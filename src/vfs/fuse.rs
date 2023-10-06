use crate::{
	artifact::Artifact, blob, directory::Directory, file::File, symlink::Symlink, template, Client,
	Error, Result, Template, WrapErr,
};
use num::ToPrimitive;
use std::{
	collections::BTreeMap,
	ffi::CString,
	io::{Read, SeekFrom, Write},
	os::{fd::FromRawFd, unix::prelude::OsStrExt},
	path::{Path, PathBuf},
	sync::{Arc, Weak},
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use zerocopy::{AsBytes, FromBytes};

/// A FUSE server.
#[derive(Clone)]
pub struct Server {
	client: Client,
	state: Arc<tokio::sync::RwLock<State>>,
}

/// The server's state.
struct State {
	nodes: BTreeMap<NodeId, Arc<Node>>,
	handles: BTreeMap<FileHandle, Arc<tokio::sync::RwLock<FileHandleData>>>,
}

/// A node in the file system.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct NodeId(pub u64);

/// The root node has ID 1.
const ROOT_NODE_ID: NodeId = NodeId(1);

/// A node.
#[derive(Debug)]
struct Node {
	id: NodeId,
	parent: Weak<Node>,
	kind: NodeKind,
}

/// An node's kind.
#[derive(Debug)]
enum NodeKind {
	Root {
		children: tokio::sync::RwLock<BTreeMap<String, Arc<Node>>>,
	},
	Directory {
		directory: Directory,
		children: tokio::sync::RwLock<BTreeMap<String, Arc<Node>>>,
	},
	File {
		file: File,
		size: u64,
	},
	Symlink {
		symlink: Symlink,
	},
}

/// A file handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct FileHandle(u64);

/// The data associated with a file handle.
enum FileHandleData {
	Directory,
	File { reader: blob::Reader },
	Symlink,
}

/// A request.
#[derive(Clone, Debug)]
struct Request {
	header: sys::fuse_in_header,
	data: RequestData,
}

/// A request's data.
#[derive(Clone, Debug)]
enum RequestData {
	Flush(sys::fuse_flush_in),
	GetAttr(sys::fuse_getattr_in),
	Init(sys::fuse_init_in),
	Lookup(CString),
	Open(sys::fuse_open_in),
	OpenDir(sys::fuse_open_in),
	Read(sys::fuse_read_in),
	ReadDir(sys::fuse_read_in),
	ReadDirPlus(sys::fuse_read_in),
	ReadLink,
	Release(sys::fuse_release_in),
	ReleaseDir(sys::fuse_release_in),
	Unsupported(u32),
}

/// A response.
#[derive(Clone, Debug)]
enum Response {
	Flush,
	GetAttr(sys::fuse_attr_out),
	Init(sys::fuse_init_out),
	Lookup(sys::fuse_entry_out),
	Open(sys::fuse_open_out),
	OpenDir(sys::fuse_open_out),
	Read(Vec<u8>),
	ReadDir(Vec<u8>),
	ReadDirPlus(Vec<u8>),
	ReadLink(CString),
	Release,
	ReleaseDir,
}

impl Server {
	/// Create a server.
	pub fn new(client: Client) -> Self {
		let root = Arc::new_cyclic(|root| Node {
			id: ROOT_NODE_ID,
			parent: root.clone(),
			kind: NodeKind::Root {
				children: tokio::sync::RwLock::new(BTreeMap::default()),
			},
		});
		let nodes = [(ROOT_NODE_ID, root)].into();
		let handles = BTreeMap::default();
		let state = State { nodes, handles };
		let state = Arc::new(tokio::sync::RwLock::new(state));
		Self { client, state }
	}

	/// Serve.
	#[allow(clippy::too_many_lines)]
	pub fn serve(self, mut fuse_file: std::fs::File) -> Result<()> {
		// Create a buffer to read requests into.
		let mut request_buffer = vec![0u8; 1024 * 1024 + 4096];

		// Handle each request.
		loop {
			// Read a request from the FUSE file.
			let request_size = match fuse_file.read(request_buffer.as_mut()) {
				Ok(request_size) => request_size,

				// Handle an error reading the request from the FUSE file.
				Err(error) => match error.raw_os_error() {
					// If the error is ENOENT, EINTR, or EAGAIN, then continue.
					Some(libc::ENOENT | libc::EINTR | libc::EAGAIN) => continue,

					// If the error is ENODEV, then the FUSE file has been unmounted.
					Some(libc::ENODEV) => return Ok(()),

					// Otherwise, return the error.
					_ => return Err(error.into()),
				},
			};
			let request_bytes = &request_buffer[..request_size];

			// Deserialize the request.
			let request_header = sys::fuse_in_header::read_from_prefix(request_bytes)
				.wrap_err("Failed to deserialize the request header.")?;
			let request_header_len = std::mem::size_of::<sys::fuse_in_header>();
			let request_data = &request_bytes[request_header_len..];
			tracing::info!(
				"FUSE request: {} ({request_size} bytes)",
				request_header.opcode
			);
			let request_data = match request_header.opcode {
				sys::fuse_opcode::FUSE_DESTROY => {
					break;
				},
				sys::fuse_opcode::FUSE_FLUSH => RequestData::Flush(read_data(request_data)?),
				sys::fuse_opcode::FUSE_GETATTR => RequestData::GetAttr(read_data(request_data)?),
				sys::fuse_opcode::FUSE_INIT => RequestData::Init(read_data(request_data)?),
				sys::fuse_opcode::FUSE_LOOKUP => {
					let data = CString::from_vec_with_nul(request_data.to_owned())
						.wrap_err("Failed to deserialize the request.")?;
					RequestData::Lookup(data)
				},
				sys::fuse_opcode::FUSE_OPEN => RequestData::Open(read_data(request_data)?),
				sys::fuse_opcode::FUSE_OPENDIR => RequestData::OpenDir(read_data(request_data)?),
				sys::fuse_opcode::FUSE_READ => RequestData::Read(read_data(request_data)?),
				sys::fuse_opcode::FUSE_READDIR => RequestData::ReadDir(read_data(request_data)?),
				sys::fuse_opcode::FUSE_READDIRPLUS => {
					RequestData::ReadDirPlus(read_data(request_data)?)
				},
				sys::fuse_opcode::FUSE_READLINK => RequestData::ReadLink,
				sys::fuse_opcode::FUSE_RELEASE => RequestData::Release(read_data(request_data)?),
				sys::fuse_opcode::FUSE_RELEASEDIR => {
					RequestData::ReleaseDir(read_data(request_data)?)
				},
				_ => RequestData::Unsupported(request_header.opcode),
			};
			let request = Request {
				header: request_header,
				data: request_data,
			};

			// Spawn a task to handle the request.
			let mut fuse_file = fuse_file.try_clone()?;
			let server = self.clone();
			tokio::spawn(async move {
				// Handle the request and get the response.
				let unique = request.header.unique;
				let result = server.handle_request(request).await;

				// Serialize the response.
				let response_bytes = match result {
					Err(error) => {
						let len = std::mem::size_of::<sys::fuse_out_header>();
						let header = sys::fuse_out_header {
							unique,
							len: len.to_u32().unwrap(),
							error: -error,
						};
						header.as_bytes().to_owned()
					},
					Ok(data) => {
						let data_bytes = match &data {
							Response::Flush | Response::Release | Response::ReleaseDir => &[],
							Response::GetAttr(data) => data.as_bytes(),
							Response::Init(data) => data.as_bytes(),
							Response::Lookup(data) => data.as_bytes(),
							Response::Open(data) | Response::OpenDir(data) => data.as_bytes(),
							Response::Read(data)
							| Response::ReadDir(data)
							| Response::ReadDirPlus(data) => data.as_bytes(),
							Response::ReadLink(data) => data.as_bytes(),
						};
						let len = std::mem::size_of::<sys::fuse_out_header>() + data_bytes.len();
						let header = sys::fuse_out_header {
							unique,
							len: len.to_u32().unwrap(),
							error: 0,
						};
						let mut buffer = header.as_bytes().to_owned();
						buffer.extend_from_slice(data_bytes);
						buffer
					},
				};

				// Write the response.
				match fuse_file.write_all(&response_bytes) {
					Ok(_) => {},
					Err(error) => {
						tracing::error!(?error, "Failed to write the response.");
					},
				};
			});
		}

		Ok(())
	}
}

fn read_data<T>(request_data: &[u8]) -> Result<T>
where
	T: FromBytes,
{
	T::read_from_prefix(request_data).wrap_err("Failed to deserialize the request data.")
}

impl Server {
	/// Handle a request.
	#[tracing::instrument(skip(self), ret)]
	async fn handle_request(&self, request: Request) -> Result<Response, i32> {
		match request.data {
			RequestData::Flush(data) => self.handle_flush_request(request.header, data).await,
			RequestData::GetAttr(data) => self.handle_get_attr_request(request.header, data).await,
			RequestData::Init(data) => self.handle_init_request(request.header, data).await,
			RequestData::Lookup(data) => self.handle_lookup_request(request.header, data).await,
			RequestData::Open(data) => self.handle_open_request(request.header, data).await,
			RequestData::OpenDir(data) => self.handle_open_dir_request(request.header, data).await,
			RequestData::Read(data) => self.handle_read_request(request.header, data).await,
			RequestData::ReadDir(data) => {
				self.handle_read_dir_request(request.header, data, false)
					.await
			},
			RequestData::ReadDirPlus(data) => {
				self.handle_read_dir_request(request.header, data, true)
					.await
			},
			RequestData::ReadLink => self.handle_read_link_request(request.header).await,
			RequestData::Release(data) => self.handle_release_request(request.header, data).await,
			RequestData::ReleaseDir(data) => {
				self.handle_release_dir_request(request.header, data).await
			},
			RequestData::Unsupported(opcode) => {
				self.handle_unsupported_request(request.header, opcode)
					.await
			},
		}
	}

	#[allow(clippy::unused_async)]
	async fn handle_flush_request(
		&self,
		_header: sys::fuse_in_header,
		_data: sys::fuse_flush_in,
	) -> Result<Response, i32> {
		Ok(Response::Flush)
	}

	async fn handle_get_attr_request(
		&self,
		header: sys::fuse_in_header,
		_data: sys::fuse_getattr_in,
	) -> Result<Response, i32> {
		let node_id = NodeId(header.nodeid);
		let node = self.get_node(node_id).await?;
		let response = node.fuse_attr_out(&self.client).await?;
		Ok(Response::GetAttr(response))
	}

	#[allow(clippy::unused_async)]
	async fn handle_init_request(
		&self,
		_header: sys::fuse_in_header,
		data: sys::fuse_init_in,
	) -> Result<Response, i32> {
		let response = sys::fuse_init_out {
			major: 7,
			minor: 21,
			max_readahead: data.max_readahead,
			flags: sys::FUSE_DO_READDIRPLUS,
			max_background: 0,
			congestion_threshold: 0,
			max_write: 1024 * 1024,
			time_gran: 0,
			max_pages: 0,
			map_alignment: 0,
			flags2: 0,
			unused: [0; 7],
		};
		Ok(Response::Init(response))
	}

	#[allow(clippy::too_many_lines)]
	async fn handle_lookup_request(
		&self,
		header: sys::fuse_in_header,
		data: CString,
	) -> Result<Response, i32> {
		// Get the parent node.
		let parent_node_id = NodeId(header.nodeid);
		let parent_node = self.get_node(parent_node_id).await?;

		// Get the name as a string.
		let Ok(name) = String::from_utf8(data.into_bytes()) else {
			return Err(libc::ENOENT);
		};

		// Get or create the child node.
		let child_node = self.get_or_create_child_node(parent_node, &name).await?;

		// Create the response.
		let response = child_node.fuse_entry_out(&self.client).await?;

		Ok(Response::Lookup(response))
	}

	#[allow(clippy::similar_names)]
	async fn handle_open_request(
		&self,
		header: sys::fuse_in_header,
		_data: sys::fuse_open_in,
	) -> Result<Response, i32> {
		// Get the node.
		let node_id = NodeId(header.nodeid);
		let node = self.get_node(node_id).await?;

		// Create the file handle.
		let file_handle_data = match &node.kind {
			NodeKind::Root { .. } => {
				tracing::error!("Cannot open the root directory.");
				return Err(libc::EPERM);
			},
			NodeKind::Directory { .. } => FileHandleData::Directory,
			NodeKind::File { file, .. } => {
				let contents = file.contents(&self.client).await.map_err(|_| libc::EIO)?;
				let Ok(reader) = contents.reader(&self.client).await else {
					tracing::error!("Failed to create reader!");
					return Err(libc::EIO);
				};
				FileHandleData::File { reader }
			},
			NodeKind::Symlink { .. } => FileHandleData::Symlink,
		};
		let file_handle_data = Arc::new(tokio::sync::RwLock::new(file_handle_data));

		// Add the file handle to the state.
		let mut state = self.state.write().await;
		let file_handle = FileHandle(state.handles.len().to_u64().unwrap() + 1);
		state.handles.insert(file_handle, file_handle_data);
		drop(state);

		// Create the response.
		let response = sys::fuse_open_out {
			fh: file_handle.0,
			open_flags: 0,
			padding: 0,
		};

		Ok(Response::Open(response))
	}

	async fn handle_open_dir_request(
		&self,
		header: sys::fuse_in_header,
		data: sys::fuse_open_in,
	) -> Result<Response, i32> {
		self.handle_open_request(header, data)
			.await
			.map(|response| match response {
				Response::Open(response) => Response::OpenDir(response),
				_ => unreachable!(),
			})
	}

	async fn handle_read_request(
		&self,
		_header: sys::fuse_in_header,
		data: sys::fuse_read_in,
	) -> Result<Response, i32> {
		let file_handle = FileHandle(data.fh);
		let file_handle_data = self
			.state
			.read()
			.await
			.handles
			.get(&file_handle)
			.ok_or(libc::ENOENT)?
			.clone();
		let mut file_handle_data = file_handle_data.write().await;
		let FileHandleData::File { reader } = &mut *file_handle_data else {
			return Err(libc::EIO);
		};
		let mut response = vec![0u8; data.size.to_usize().unwrap()];
		reader
			.seek(SeekFrom::Start(data.offset.to_u64().unwrap()))
			.await
			.map_err(|_| libc::EIO)?;
		let n = reader.read(&mut response).await.map_err(|_| libc::EIO)?;
		response.truncate(n);
		Ok(Response::Read(response))
	}

	#[allow(clippy::unused_async)]
	async fn handle_read_dir_request(
		&self,
		header: sys::fuse_in_header,
		data: sys::fuse_read_in,
		plus: bool,
	) -> Result<Response, i32> {
		// Get the node.
		let node_id = NodeId(header.nodeid);
		let node = self.get_node(node_id).await?;

		// If the node is the root, then return an empty response.
		if let NodeKind::Root { .. } = &node.kind {
			return Ok(Response::ReadDir(vec![]));
		};

		// Otherwise, the node must be a directory.
		let NodeKind::Directory { directory, .. } = &node.kind else {
			return Err(libc::EIO);
		};

		// Create the response.
		let mut response = Vec::new();
		let names = directory
			.entries(&self.client)
			.await
			.map_err(|_| libc::EIO)?
			.keys()
			.map(|k| k.as_ref());

		for (offset, name) in [".", ".."]
			.into_iter()
			.chain(names)
			.enumerate()
			.skip(data.offset.to_usize().unwrap())
		{
			// Get the node.
			let node = match name {
				"." => node.clone(),
				".." => node.parent.upgrade().unwrap(),
				_ => self.get_or_create_child_node(node.clone(), name).await?,
			};

			// Compute the padding entry size.
			let struct_size = if plus {
				std::mem::size_of::<sys::fuse_direntplus>()
			} else {
				std::mem::size_of::<sys::fuse_dirent>()
			};
			let padding = (8 - (struct_size + name.len()) % 8) % 8;
			let entry_size = struct_size + name.len() + padding;

			// If the response will exceed the specified size, then break.
			if response.len() + entry_size > data.size.to_usize().unwrap() {
				break;
			}

			// Otherwise, add the entry.
			let entry = sys::fuse_dirent {
				ino: node.id.0,
				off: offset.to_u64().unwrap() + 1,
				namelen: name.len().to_u32().unwrap(),
				type_: node.type_(),
			};
			if plus {
				let entry = sys::fuse_direntplus {
					entry_out: node.fuse_entry_out(&self.client).await?,
					dirent: entry,
				};
				response.extend_from_slice(entry.as_bytes());
			} else {
				response.extend_from_slice(entry.as_bytes());
			};
			response.extend_from_slice(name.as_bytes());
			response.extend((0..padding).map(|_| 0));
		}

		Ok(if plus {
			Response::ReadDirPlus(response)
		} else {
			Response::ReadDir(response)
		})
	}

	#[allow(clippy::unused_async)]
	async fn handle_read_link_request(&self, header: sys::fuse_in_header) -> Result<Response, i32> {
		// Get the node.
		let node_id = NodeId(header.nodeid);
		let node = self.get_node(node_id).await?;

		// Get the target.
		let target: Template = match &node.kind {
			NodeKind::Symlink { symlink, .. } => {
				symlink.target(&self.client).await.map_err(|_| libc::EIO)?
			},
			_ => return Err(libc::EIO),
		};

		// Render the target.
		let mut response = String::new();
		for component in target.components() {
			match component {
				template::Component::String(string) => response.push_str(string),
				template::Component::Artifact(artifact) => {
					let id = artifact.id(&self.client).await.map_err(|_| libc::EIO)?;
					for _ in 0..node.depth() {
						response.push_str("../");
					}
					response.push_str(&id.to_string());
				},
				template::Component::Placeholder(_) => {
					return Err(libc::EIO);
				},
			}
		}
		let response = CString::new(response).unwrap();

		Ok(Response::ReadLink(response))
	}

	async fn handle_release_request(
		&self,
		_header: sys::fuse_in_header,
		data: sys::fuse_release_in,
	) -> Result<Response, i32> {
		let file_handle = FileHandle(data.fh);
		self.state.write().await.handles.remove(&file_handle);
		Ok(Response::Release)
	}

	async fn handle_release_dir_request(
		&self,
		_header: sys::fuse_in_header,
		data: sys::fuse_release_in,
	) -> Result<Response, i32> {
		let file_handle = FileHandle(data.fh);
		self.state.write().await.handles.remove(&file_handle);
		Ok(Response::ReleaseDir)
	}

	#[allow(clippy::unused_async)]
	async fn handle_unsupported_request(
		&self,
		_header: sys::fuse_in_header,
		opcode: u32,
	) -> Result<Response, i32> {
		if opcode == sys::fuse_opcode::FUSE_IOCTL {
			return Err(libc::ENOTTY);
		}
		tracing::error!(?opcode, "Unsupported FUSE request.");
		Err(libc::ENOSYS)
	}
}

impl Server {
	async fn get_node(&self, node_id: NodeId) -> Result<Arc<Node>, i32> {
		let state = self.state.read().await;
		let Some(node) = state.nodes.get(&node_id).cloned() else {
			return Err(libc::ENOENT);
		};
		Ok(node)
	}

	async fn get_or_create_child_node(
		&self,
		parent_node: Arc<Node>,
		name: &str,
	) -> Result<Arc<Node>, i32> {
		// Handle ".".
		if name == "." {
			return Ok(parent_node);
		}

		// Handle "..".
		if name == ".." {
			let parent_parent_node = parent_node.parent.upgrade().ok_or(libc::EIO)?;
			return Ok(parent_parent_node);
		}

		// If the child already exists, then return it.
		match &parent_node.kind {
			NodeKind::Root { children } | NodeKind::Directory { children, .. } => {
				if let Some(child) = children.read().await.get(name).cloned() {
					return Ok(child);
				}
			},

			_ => return Err(libc::EIO),
		}

		// Get the child artifact.
		let child_artifact = match &parent_node.kind {
			NodeKind::Root { .. } => {
				let id = name.parse().map_err(|_| libc::ENOENT)?;
				Artifact::with_id(id)
			},

			NodeKind::Directory { directory, .. } => {
				let entries = directory.entries(&self.client).await.map_err(|e| {
					tracing::error!(?e, "Failed to get directory entries.");
					libc::EIO
				})?;
				entries.get(name).ok_or(libc::ENOENT)?.clone()
			},

			_ => return Err(libc::EIO),
		};

		// Create the child node.
		let node_id = NodeId(self.state.read().await.nodes.len() as u64 + 1000);
		let kind = match child_artifact {
			Artifact::Directory(directory) => {
				let children = tokio::sync::RwLock::new(BTreeMap::default());
				NodeKind::Directory {
					directory,
					children,
				}
			},
			Artifact::File(file) => {
				let contents = file.contents(&self.client).await.map_err(|_| libc::EIO)?;
				let size = contents.size(&self.client).await.map_err(|_| libc::EIO)?;
				NodeKind::File { file, size }
			},
			Artifact::Symlink(symlink) => NodeKind::Symlink { symlink },
		};
		let child_node = Node {
			id: node_id,
			parent: Arc::downgrade(&parent_node),
			kind,
		};
		let child_node = Arc::new(child_node);

		// Add the child node to the parent node.
		match &parent_node.kind {
			NodeKind::Root { children } | NodeKind::Directory { children, .. } => {
				children
					.write()
					.await
					.insert(name.to_owned(), child_node.clone());
			},

			_ => return Err(libc::EIO),
		}

		// Add the child node to the nodes.
		self.state
			.write()
			.await
			.nodes
			.insert(child_node.id, child_node.clone());

		Ok(child_node)
	}
}

impl Node {
	fn type_(&self) -> u32 {
		match &self.kind {
			NodeKind::Root { .. } | NodeKind::Directory { .. } => libc::S_IFDIR as _,
			NodeKind::File { .. } => libc::S_IFREG as _,
			NodeKind::Symlink { .. } => libc::S_IFLNK as _,
		}
	}

	async fn mode(&self, client: &Client) -> Result<u32, i32> {
		let mode = match &self.kind {
			NodeKind::Root { .. } | NodeKind::Directory { .. } => libc::S_IFDIR | 0o555,
			NodeKind::File { file, .. } => {
				let executable = file.executable(client).await.map_err(|_| libc::EIO)?;
				libc::S_IFREG | 0o444 | (if executable { 0o111 } else { 0o000 })
			},
			NodeKind::Symlink { .. } => libc::S_IFLNK | 0o444,
		};
		Ok(mode as _)
	}

	fn size(&self) -> u64 {
		match &self.kind {
			NodeKind::Root { .. } | NodeKind::Directory { .. } | NodeKind::Symlink { .. } => 0,
			NodeKind::File { size, .. } => *size,
		}
	}

	async fn fuse_entry_out(&self, client: &Client) -> Result<sys::fuse_entry_out, i32> {
		let nodeid = self.id.0;
		let attr_out = self.fuse_attr_out(client).await?;
		let entry_out = sys::fuse_entry_out {
			nodeid,
			generation: 0,
			entry_valid: 1024,
			attr_valid: 0,
			entry_valid_nsec: 1024,
			attr_valid_nsec: 0,
			attr: attr_out.attr,
		};
		Ok(entry_out)
	}

	async fn fuse_attr_out(&self, client: &Client) -> Result<sys::fuse_attr_out, i32> {
		let nodeid = self.id.0;
		let nlink: u32 = match &self.kind {
			NodeKind::Root { .. } => 2,
			_ => 1,
		};
		let size = self.size();
		let mode = self.mode(client).await?;
		let attr_out = sys::fuse_attr_out {
			attr_valid: 1024,
			attr_valid_nsec: 0,
			attr: sys::fuse_attr {
				ino: nodeid,
				size,
				blocks: 0,
				atime: 0,
				mtime: 0,
				ctime: 0,
				atimensec: 0,
				mtimensec: 0,
				ctimensec: 0,
				mode,
				nlink,
				uid: 1000,
				gid: 1000,
				rdev: 0,
				blksize: 512,
				flags: 0,
			},
			dummy: 0,
		};
		Ok(attr_out)
	}
}

mod sys {
	#![allow(warnings)]

	pub const FUSE_KERNEL_VERSION: u32 = 7;
	pub const FUSE_KERNEL_MINOR_VERSION: u32 = 38;
	pub const FUSE_ROOT_ID: u32 = 1;
	pub const FATTR_MODE: u32 = 1;
	pub const FATTR_UID: u32 = 2;
	pub const FATTR_GID: u32 = 4;
	pub const FATTR_SIZE: u32 = 8;
	pub const FATTR_ATIME: u32 = 16;
	pub const FATTR_MTIME: u32 = 32;
	pub const FATTR_FH: u32 = 64;
	pub const FATTR_ATIME_NOW: u32 = 128;
	pub const FATTR_MTIME_NOW: u32 = 256;
	pub const FATTR_LOCKOWNER: u32 = 512;
	pub const FATTR_CTIME: u32 = 1024;
	pub const FATTR_KILL_SUIDGID: u32 = 2048;
	pub const FOPEN_DIRECT_IO: u32 = 1;
	pub const FOPEN_KEEP_CACHE: u32 = 2;
	pub const FOPEN_NONSEEKABLE: u32 = 4;
	pub const FOPEN_CACHE_DIR: u32 = 8;
	pub const FOPEN_STREAM: u32 = 16;
	pub const FOPEN_NOFLUSH: u32 = 32;
	pub const FOPEN_PARALLEL_DIRECT_WRITES: u32 = 64;
	pub const FUSE_ASYNC_READ: u32 = 1;
	pub const FUSE_POSIX_LOCKS: u32 = 2;
	pub const FUSE_FILE_OPS: u32 = 4;
	pub const FUSE_ATOMIC_O_TRUNC: u32 = 8;
	pub const FUSE_EXPORT_SUPPORT: u32 = 16;
	pub const FUSE_BIG_WRITES: u32 = 32;
	pub const FUSE_DONT_MASK: u32 = 64;
	pub const FUSE_SPLICE_WRITE: u32 = 128;
	pub const FUSE_SPLICE_MOVE: u32 = 256;
	pub const FUSE_SPLICE_READ: u32 = 512;
	pub const FUSE_FLOCK_LOCKS: u32 = 1024;
	pub const FUSE_HAS_IOCTL_DIR: u32 = 2048;
	pub const FUSE_AUTO_INVAL_DATA: u32 = 4096;
	pub const FUSE_DO_READDIRPLUS: u32 = 8192;
	pub const FUSE_READDIRPLUS_AUTO: u32 = 16384;
	pub const FUSE_ASYNC_DIO: u32 = 32768;
	pub const FUSE_WRITEBACK_CACHE: u32 = 65536;
	pub const FUSE_NO_OPEN_SUPPORT: u32 = 131072;
	pub const FUSE_PARALLEL_DIROPS: u32 = 262144;
	pub const FUSE_HANDLE_KILLPRIV: u32 = 524288;
	pub const FUSE_POSIX_ACL: u32 = 1048576;
	pub const FUSE_ABORT_ERROR: u32 = 2097152;
	pub const FUSE_MAX_PAGES: u32 = 4194304;
	pub const FUSE_CACHE_SYMLINKS: u32 = 8388608;
	pub const FUSE_NO_OPENDIR_SUPPORT: u32 = 16777216;
	pub const FUSE_EXPLICIT_INVAL_DATA: u32 = 33554432;
	pub const FUSE_MAP_ALIGNMENT: u32 = 67108864;
	pub const FUSE_SUBMOUNTS: u32 = 134217728;
	pub const FUSE_HANDLE_KILLPRIV_V2: u32 = 268435456;
	pub const FUSE_SETXATTR_EXT: u32 = 536870912;
	pub const FUSE_INIT_EXT: u32 = 1073741824;
	pub const FUSE_INIT_RESERVED: u32 = 2147483648;
	pub const FUSE_SECURITY_CTX: u64 = 4294967296;
	pub const FUSE_HAS_INODE_DAX: u64 = 8589934592;
	pub const CUSE_UNRESTRICTED_IOCTL: u32 = 1;
	pub const FUSE_RELEASE_FLUSH: u32 = 1;
	pub const FUSE_RELEASE_FLOCK_UNLOCK: u32 = 2;
	pub const FUSE_GETATTR_FH: u32 = 1;
	pub const FUSE_LK_FLOCK: u32 = 1;
	pub const FUSE_WRITE_CACHE: u32 = 1;
	pub const FUSE_WRITE_LOCKOWNER: u32 = 2;
	pub const FUSE_WRITE_KILL_SUIDGID: u32 = 4;
	pub const FUSE_WRITE_KILL_PRIV: u32 = 4;
	pub const FUSE_READ_LOCKOWNER: u32 = 2;
	pub const FUSE_IOCTL_COMPAT: u32 = 1;
	pub const FUSE_IOCTL_UNRESTRICTED: u32 = 2;
	pub const FUSE_IOCTL_RETRY: u32 = 4;
	pub const FUSE_IOCTL_32BIT: u32 = 8;
	pub const FUSE_IOCTL_DIR: u32 = 16;
	pub const FUSE_IOCTL_COMPAT_X32: u32 = 32;
	pub const FUSE_IOCTL_MAX_IOV: u32 = 256;
	pub const FUSE_POLL_SCHEDULE_NOTIFY: u32 = 1;
	pub const FUSE_FSYNC_FDATASYNC: u32 = 1;
	pub const FUSE_ATTR_SUBMOUNT: u32 = 1;
	pub const FUSE_ATTR_DAX: u32 = 2;
	pub const FUSE_OPEN_KILL_SUIDGID: u32 = 1;
	pub const FUSE_SETXATTR_ACL_KILL_SGID: u32 = 1;
	pub const FUSE_EXPIRE_ONLY: u32 = 1;
	pub const FUSE_MIN_READ_BUFFER: u32 = 8192;
	pub const FUSE_COMPAT_ENTRY_OUT_SIZE: u32 = 120;
	pub const FUSE_COMPAT_ATTR_OUT_SIZE: u32 = 96;
	pub const FUSE_COMPAT_MKNOD_IN_SIZE: u32 = 8;
	pub const FUSE_COMPAT_WRITE_IN_SIZE: u32 = 24;
	pub const FUSE_COMPAT_STATFS_SIZE: u32 = 48;
	pub const FUSE_COMPAT_SETXATTR_IN_SIZE: u32 = 8;
	pub const FUSE_COMPAT_INIT_OUT_SIZE: u32 = 8;
	pub const FUSE_COMPAT_22_INIT_OUT_SIZE: u32 = 24;
	pub const CUSE_INIT_INFO_MAX: u32 = 4096;
	pub const FUSE_DEV_IOC_MAGIC: u32 = 229;
	pub const FUSE_SETUPMAPPING_FLAG_WRITE: u32 = 1;
	pub const FUSE_SETUPMAPPING_FLAG_READ: u32 = 2;

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_attr {
		pub ino: u64,
		pub size: u64,
		pub blocks: u64,
		pub atime: u64,
		pub mtime: u64,
		pub ctime: u64,
		pub atimensec: u32,
		pub mtimensec: u32,
		pub ctimensec: u32,
		pub mode: u32,
		pub nlink: u32,
		pub uid: u32,
		pub gid: u32,
		pub rdev: u32,
		pub blksize: u32,
		pub flags: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_kstatfs {
		pub blocks: u64,
		pub bfree: u64,
		pub bavail: u64,
		pub files: u64,
		pub ffree: u64,
		pub bsize: u32,
		pub namelen: u32,
		pub frsize: u32,
		pub padding: u32,
		pub spare: [u32; 6usize],
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_file_lock {
		pub start: u64,
		pub end: u64,
		pub type_: u32,
		pub pid: u32,
	}

	pub mod fuse_opcode {
		pub type Type = ::std::os::raw::c_uint;
		pub const FUSE_LOOKUP: Type = 1;
		pub const FUSE_FORGET: Type = 2;
		pub const FUSE_GETATTR: Type = 3;
		pub const FUSE_SETATTR: Type = 4;
		pub const FUSE_READLINK: Type = 5;
		pub const FUSE_SYMLINK: Type = 6;
		pub const FUSE_MKNOD: Type = 8;
		pub const FUSE_MKDIR: Type = 9;
		pub const FUSE_UNLINK: Type = 10;
		pub const FUSE_RMDIR: Type = 11;
		pub const FUSE_RENAME: Type = 12;
		pub const FUSE_LINK: Type = 13;
		pub const FUSE_OPEN: Type = 14;
		pub const FUSE_READ: Type = 15;
		pub const FUSE_WRITE: Type = 16;
		pub const FUSE_STATFS: Type = 17;
		pub const FUSE_RELEASE: Type = 18;
		pub const FUSE_FSYNC: Type = 20;
		pub const FUSE_SETXATTR: Type = 21;
		pub const FUSE_GETXATTR: Type = 22;
		pub const FUSE_LISTXATTR: Type = 23;
		pub const FUSE_REMOVEXATTR: Type = 24;
		pub const FUSE_FLUSH: Type = 25;
		pub const FUSE_INIT: Type = 26;
		pub const FUSE_OPENDIR: Type = 27;
		pub const FUSE_READDIR: Type = 28;
		pub const FUSE_RELEASEDIR: Type = 29;
		pub const FUSE_FSYNCDIR: Type = 30;
		pub const FUSE_GETLK: Type = 31;
		pub const FUSE_SETLK: Type = 32;
		pub const FUSE_SETLKW: Type = 33;
		pub const FUSE_ACCESS: Type = 34;
		pub const FUSE_CREATE: Type = 35;
		pub const FUSE_INTERRUPT: Type = 36;
		pub const FUSE_BMAP: Type = 37;
		pub const FUSE_DESTROY: Type = 38;
		pub const FUSE_IOCTL: Type = 39;
		pub const FUSE_POLL: Type = 40;
		pub const FUSE_NOTIFY_REPLY: Type = 41;
		pub const FUSE_BATCH_FORGET: Type = 42;
		pub const FUSE_FALLOCATE: Type = 43;
		pub const FUSE_READDIRPLUS: Type = 44;
		pub const FUSE_RENAME2: Type = 45;
		pub const FUSE_LSEEK: Type = 46;
		pub const FUSE_COPY_FILE_RANGE: Type = 47;
		pub const FUSE_SETUPMAPPING: Type = 48;
		pub const FUSE_REMOVEMAPPING: Type = 49;
		pub const FUSE_SYNCFS: Type = 50;
		pub const FUSE_TMPFILE: Type = 51;
		pub const CUSE_INIT: Type = 4096;
		pub const CUSE_INIT_BSWAP_RESERVED: Type = 1048576;
		pub const FUSE_INIT_BSWAP_RESERVED: Type = 436207616;
	}

	pub mod fuse_notify_code {
		pub type Type = ::std::os::raw::c_uint;
		pub const FUSE_NOTIFY_POLL: Type = 1;
		pub const FUSE_NOTIFY_INVAL_INODE: Type = 2;
		pub const FUSE_NOTIFY_INVAL_ENTRY: Type = 3;
		pub const FUSE_NOTIFY_STORE: Type = 4;
		pub const FUSE_NOTIFY_RETRIEVE: Type = 5;
		pub const FUSE_NOTIFY_DELETE: Type = 6;
		pub const FUSE_NOTIFY_CODE_MAX: Type = 7;
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_entry_out {
		pub nodeid: u64,
		pub generation: u64,
		pub entry_valid: u64,
		pub attr_valid: u64,
		pub entry_valid_nsec: u32,
		pub attr_valid_nsec: u32,
		pub attr: fuse_attr,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_forget_in {
		pub nlookup: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_forget_one {
		pub nodeid: u64,
		pub nlookup: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_batch_forget_in {
		pub count: u32,
		pub dummy: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_getattr_in {
		pub getattr_flags: u32,
		pub dummy: u32,
		pub fh: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_attr_out {
		pub attr_valid: u64,
		pub attr_valid_nsec: u32,
		pub dummy: u32,
		pub attr: fuse_attr,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_mknod_in {
		pub mode: u32,
		pub rdev: u32,
		pub umask: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_mkdir_in {
		pub mode: u32,
		pub umask: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_rename_in {
		pub newdir: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_rename2_in {
		pub newdir: u64,
		pub flags: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_link_in {
		pub oldnodeid: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_setattr_in {
		pub valid: u32,
		pub padding: u32,
		pub fh: u64,
		pub size: u64,
		pub lock_owner: u64,
		pub atime: u64,
		pub mtime: u64,
		pub ctime: u64,
		pub atimensec: u32,
		pub mtimensec: u32,
		pub ctimensec: u32,
		pub mode: u32,
		pub unused4: u32,
		pub uid: u32,
		pub gid: u32,
		pub unused5: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_open_in {
		pub flags: u32,
		pub open_flags: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_create_in {
		pub flags: u32,
		pub mode: u32,
		pub umask: u32,
		pub open_flags: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_open_out {
		pub fh: u64,
		pub open_flags: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_release_in {
		pub fh: u64,
		pub flags: u32,
		pub release_flags: u32,
		pub lock_owner: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_flush_in {
		pub fh: u64,
		pub unused: u32,
		pub padding: u32,
		pub lock_owner: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_read_in {
		pub fh: u64,
		pub offset: u64,
		pub size: u32,
		pub read_flags: u32,
		pub lock_owner: u64,
		pub flags: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_write_in {
		pub fh: u64,
		pub offset: u64,
		pub size: u32,
		pub write_flags: u32,
		pub lock_owner: u64,
		pub flags: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_write_out {
		pub size: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_statfs_out {
		pub st: fuse_kstatfs,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_fsync_in {
		pub fh: u64,
		pub fsync_flags: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_setxattr_in {
		pub size: u32,
		pub flags: u32,
		pub setxattr_flags: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_getxattr_in {
		pub size: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_getxattr_out {
		pub size: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_lk_in {
		pub fh: u64,
		pub owner: u64,
		pub lk: fuse_file_lock,
		pub lk_flags: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_lk_out {
		pub lk: fuse_file_lock,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_access_in {
		pub mask: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_init_in {
		pub major: u32,
		pub minor: u32,
		pub max_readahead: u32,
		pub flags: u32,
		pub flags2: u32,
		pub unused: [u32; 11usize],
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_init_out {
		pub major: u32,
		pub minor: u32,
		pub max_readahead: u32,
		pub flags: u32,
		pub max_background: u16,
		pub congestion_threshold: u16,
		pub max_write: u32,
		pub time_gran: u32,
		pub max_pages: u16,
		pub map_alignment: u16,
		pub flags2: u32,
		pub unused: [u32; 7usize],
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct cuse_init_in {
		pub major: u32,
		pub minor: u32,
		pub unused: u32,
		pub flags: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct cuse_init_out {
		pub major: u32,
		pub minor: u32,
		pub unused: u32,
		pub flags: u32,
		pub max_read: u32,
		pub max_write: u32,
		pub dev_major: u32,
		pub dev_minor: u32,
		pub spare: [u32; 10usize],
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_interrupt_in {
		pub unique: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_bmap_in {
		pub block: u64,
		pub blocksize: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_bmap_out {
		pub block: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_ioctl_in {
		pub fh: u64,
		pub flags: u32,
		pub cmd: u32,
		pub arg: u64,
		pub in_size: u32,
		pub out_size: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_ioctl_iovec {
		pub base: u64,
		pub len: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_ioctl_out {
		pub result: i32,
		pub flags: u32,
		pub in_iovs: u32,
		pub out_iovs: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_poll_in {
		pub fh: u64,
		pub kh: u64,
		pub flags: u32,
		pub events: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_poll_out {
		pub revents: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_notify_poll_wakeup_out {
		pub kh: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_fallocate_in {
		pub fh: u64,
		pub offset: u64,
		pub length: u64,
		pub mode: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_in_header {
		pub len: u32,
		pub opcode: u32,
		pub unique: u64,
		pub nodeid: u64,
		pub uid: u32,
		pub gid: u32,
		pub pid: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_out_header {
		pub len: u32,
		pub error: i32,
		pub unique: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_dirent {
		pub ino: u64,
		pub off: u64,
		pub namelen: u32,
		pub type_: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_direntplus {
		pub entry_out: fuse_entry_out,
		pub dirent: fuse_dirent,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_notify_inval_inode_out {
		pub ino: u64,
		pub off: i64,
		pub len: i64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_notify_inval_entry_out {
		pub parent: u64,
		pub namelen: u32,
		pub flags: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_notify_delete_out {
		pub parent: u64,
		pub child: u64,
		pub namelen: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_notify_store_out {
		pub nodeid: u64,
		pub offset: u64,
		pub size: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_notify_retrieve_out {
		pub notify_unique: u64,
		pub nodeid: u64,
		pub offset: u64,
		pub size: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_notify_retrieve_in {
		pub dummy1: u64,
		pub offset: u64,
		pub size: u32,
		pub dummy2: u32,
		pub dummy3: u64,
		pub dummy4: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_lseek_in {
		pub fh: u64,
		pub offset: u64,
		pub whence: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_lseek_out {
		pub offset: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_copy_file_range_in {
		pub fh_in: u64,
		pub off_in: u64,
		pub nodeid_out: u64,
		pub fh_out: u64,
		pub off_out: u64,
		pub len: u64,
		pub flags: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_setupmapping_in {
		pub fh: u64,
		pub foffset: u64,
		pub len: u64,
		pub flags: u64,
		pub moffset: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_removemapping_in {
		pub count: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_removemapping_one {
		pub moffset: u64,
		pub len: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_syncfs_in {
		pub padding: u64,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_secctx {
		pub size: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Clone, Debug, zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
	pub struct fuse_secctx_header {
		pub size: u32,
		pub nr_secctx: u32,
	}
}

pub async fn mount(mountpoint: PathBuf) -> crate::Result<std::fs::File> {
	unmount(&mountpoint).await?;
	let result = unsafe { mount_inner(&mountpoint) };
	if result.is_err() {
		let _ = unmount(&mountpoint).await;
	}
	result
}

async fn unmount(mountpoint: &Path) -> crate::Result<()> {
	tokio::process::Command::new("fusermount3")
		.arg("-q")
		.arg("-u")
		.arg(mountpoint)
		.status()
		.await?;
	Ok(())
}

unsafe fn mount_inner(mountpoint: &Path) -> crate::Result<std::fs::File> {
	// Setup the arguments.
	let uid = libc::getuid();
	let gid = libc::getgid();
	let options =
		format!("rootmode=40755,user_id={uid},group_id={gid},default_permissions,auto_unmount\0");

	let mut fds = [0, 0];
	let ec = libc::socketpair(libc::PF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr());
	if ec != 0 {
		tracing::error!("socketpair() failed.");
		Err(std::io::Error::last_os_error())?;
	}

	let fusermount3 = std::ffi::CString::new("/usr/bin/fusermount3").unwrap();
	let fuse_commfd = std::ffi::CString::new(fds[0].to_string()).unwrap();

	// Fork.
	let pid = libc::fork();
	if pid == -1 {
		tracing::error!("fork() failed.");
		libc::close(fds[0]);
		libc::close(fds[1]);
		Err(std::io::Error::last_os_error())?;
	}

	// Exec the program.
	if pid == 0 {
		let argv = [
			fusermount3.as_ptr(),
			b"-o\0".as_ptr().cast(),
			options.as_ptr().cast(),
			b"--\0".as_ptr().cast(),
			mountpoint.as_os_str().as_bytes().as_ptr().cast(),
			std::ptr::null(),
		];
		libc::close(fds[1]);
		libc::fcntl(fds[0], libc::F_SETFD, 0);
		libc::setenv(
			b"_FUSE_COMMFD\0".as_ptr().cast(),
			fuse_commfd.as_ptr().cast(),
			1,
		);
		libc::execv(argv[0], argv.as_ptr());
		libc::perror(b"tangram: failed to mount fuse\0".as_ptr().cast());
		libc::close(fds[0]);
		libc::exit(1);
	}
	libc::close(fds[0]);

	// RECVFD
	// Create the control message.
	let mut control = [0u8; unsafe { libc::CMSG_SPACE(4) as usize }];
	let mut msg = libc::msghdr {
		msg_name: std::ptr::null_mut(),
		msg_namelen: 0,
		msg_iov: [libc::iovec {
			iov_base: [0u8; 8].as_mut_ptr().cast(),
			iov_len: 8,
		}]
		.as_mut_ptr(),
		msg_iovlen: 1,
		msg_control: control.as_mut_ptr().cast(),
		msg_controllen: std::mem::size_of_val(&control) as _,
		msg_flags: 0,
	};

	// Receive the message.
	tracing::info!("Calling recvmsg");
	let ret = libc::recvmsg(fds[1], std::ptr::addr_of_mut!(msg), 0);
	if ret == -1 {
		return Err(std::io::Error::last_os_error().into());
	}
	if ret == 0 {
		return Err(
			std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Unexpected EOF.").into(),
		);
	}

	// Read the file descriptor.
	let cmsg = libc::CMSG_FIRSTHDR(std::ptr::addr_of_mut!(msg));
	if cmsg.is_null() {
		return Err(
			std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Unexpected EOF.").into(),
		);
	}
	let mut fd: std::os::unix::io::RawFd = 0;
	libc::memcpy(
		std::ptr::addr_of_mut!(fd).cast(),
		libc::CMSG_DATA(cmsg).cast(),
		std::mem::size_of_val(&fd),
	);
	tracing::info!("Got /dev/fuse fd: {fd}");

	// let mut status = 0;
	// libc::waitpid(pid, std::ptr::addr_of_mut!(status), 0);
	// tracing::info!("fusermount3 exited with status {status}.");
	if fd > 0 {
		libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
	}

	Ok(std::fs::File::from_raw_fd(fd))
}

impl Node {
	fn depth(self: &Arc<Self>) -> usize {
		if self.id == ROOT_NODE_ID {
			0
		} else {
			1 + self.parent.upgrade().unwrap().depth()
		}
	}
}
