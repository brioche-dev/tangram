use num::ToPrimitive;
use std::{
	collections::BTreeMap,
	ffi::CString,
	io::SeekFrom,
	os::{fd::FromRawFd, unix::prelude::OsStrExt},
	path::Path,
	sync::{Arc, Weak},
};
use tangram_client as tg;
use tg::{
	artifact::Artifact, blob, directory::Directory, file::File, symlink::Symlink, template, Client,
	Result, Template, Wrap, WrapErr,
};
use tokio::io::AsyncWriteExt;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use zerocopy::{AsBytes, FromBytes};

mod sys;

/// A FUSE server.
#[derive(Clone)]
pub struct Server {
	client: Arc<dyn Client>,
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
	pub fn new(client: &dyn tg::Client) -> Self {
		let client = Arc::from(client.clone_box());
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
	pub async fn serve(&self, fuse_file: std::fs::File) -> Result<()> {
		let mut fuse_file = tokio::fs::File::from_std(fuse_file);

		// Create a buffer to read requests into.
		let mut request_buffer = vec![0u8; 1024 * 1024 + 4096];

		// Handle each request.
		loop {
			// Read a request from the FUSE file.
			let request_size = match fuse_file.read(request_buffer.as_mut()).await {
				Ok(request_size) => request_size,

				// Handle an error reading the request from the FUSE file.
				Err(error) => match error.raw_os_error() {
					// If the error is ENOENT, EINTR, or EAGAIN, then continue.
					Some(libc::ENOENT | libc::EINTR | libc::EAGAIN) => continue,

					// If the error is ENODEV, then the FUSE file has been unmounted.
					Some(libc::ENODEV) => return Ok(()),

					// Otherwise, return the error.
					_ => return Err(error.wrap("Failed to read the request.")),
				},
			};
			let request_bytes = &request_buffer[..request_size];

			// Deserialize the request.
			let request_header = sys::fuse_in_header::read_from_prefix(request_bytes)
				.wrap_err("Failed to deserialize the request header.")?;
			let request_header_len = std::mem::size_of::<sys::fuse_in_header>();
			let request_data = &request_bytes[request_header_len..];
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
			let mut fuse_file = fuse_file
				.try_clone()
				.await
				.wrap_err("Failed to clone the FUSE file.")?;
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
				match fuse_file.write_all(&response_bytes).await {
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
		let response = node.fuse_attr_out(self.client.as_ref()).await?;
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
		let response = child_node.fuse_entry_out(self.client.as_ref()).await?;

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
				return Err(libc::EPERM);
			},
			NodeKind::Directory { .. } => FileHandleData::Directory,
			NodeKind::File { file, .. } => {
				let contents = file
					.contents(self.client.as_ref())
					.await
					.map_err(|_| libc::EIO)?;
				let Ok(reader) = contents.reader(self.client.as_ref()).await else {
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
			.entries(self.client.as_ref())
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
					entry_out: node.fuse_entry_out(self.client.as_ref()).await?,
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
			NodeKind::Symlink { symlink, .. } => symlink
				.target(self.client.as_ref())
				.await
				.map_err(|_| libc::EIO)?,
			_ => return Err(libc::EIO),
		};

		// Render the target.
		let mut response = String::new();
		for component in target.components() {
			match component {
				template::Component::String(string) => response.push_str(string),
				template::Component::Artifact(artifact) => {
					let id = artifact
						.id(self.client.as_ref())
						.await
						.map_err(|_| libc::EIO)?;
					for _ in 0..node.depth() {
						response.push_str("../");
					}
					response.push_str(&id.to_string());
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
				let entries = directory
					.entries(self.client.as_ref())
					.await
					.map_err(|_| libc::EIO)?;
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
				let contents = file
					.contents(self.client.as_ref())
					.await
					.map_err(|_| libc::EIO)?;
				let size = contents
					.size(self.client.as_ref())
					.await
					.map_err(|_| libc::EIO)?;
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

	async fn mode(&self, client: &dyn Client) -> Result<u32, i32> {
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

	fn depth(&self) -> usize {
		if self.id == ROOT_NODE_ID {
			0
		} else {
			1 + self.parent.upgrade().unwrap().depth()
		}
	}

	async fn fuse_entry_out(&self, client: &dyn Client) -> Result<sys::fuse_entry_out, i32> {
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

	async fn fuse_attr_out(&self, client: &dyn Client) -> Result<sys::fuse_attr_out, i32> {
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

pub async fn mount(path: &Path) -> crate::Result<std::fs::File> {
	unmount(path).await?;
	let result = unsafe { mount_inner(path) };
	if result.is_err() {
		unmount(path).await?;
	}
	result
}

unsafe fn mount_inner(path: &Path) -> crate::Result<std::fs::File> {
	// Setup the arguments.
	let uid = libc::getuid();
	let gid = libc::getgid();
	let options =
		format!("rootmode=40755,user_id={uid},group_id={gid},default_permissions,auto_unmount\0");

	let mut fds = [0, 0];
	let ret = libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr());
	if ret != 0 {
		Err(std::io::Error::last_os_error()).wrap_err("Failed to create the socket pair.")?;
	}

	let fusermount3 = std::ffi::CString::new("/usr/bin/fusermount3").unwrap();
	let fuse_commfd = std::ffi::CString::new(fds[0].to_string()).unwrap();

	// Fork.
	let pid = libc::fork();
	if pid == -1 {
		libc::close(fds[0]);
		libc::close(fds[1]);
		Err(std::io::Error::last_os_error()).wrap_err("Failed to fork.")?;
	}

	// Exec the program.
	if pid == 0 {
		let argv = [
			fusermount3.as_ptr(),
			b"-o\0".as_ptr().cast(),
			options.as_ptr().cast(),
			b"--\0".as_ptr().cast(),
			path.as_os_str().as_bytes().as_ptr().cast(),
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

	// Receive the control message.
	let ret = libc::recvmsg(fds[1], std::ptr::addr_of_mut!(msg), 0);
	if ret == -1 {
		return Err(std::io::Error::last_os_error().wrap("Failed to receive the message."));
	}
	if ret == 0 {
		return Err(
			std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Unexpected EOF.")
				.wrap("Unexpected EOF."),
		);
	}

	// Read the file descriptor.
	let cmsg = libc::CMSG_FIRSTHDR(std::ptr::addr_of_mut!(msg));
	if cmsg.is_null() {
		return Err(
			std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Unexpected EOF.")
				.wrap("Unexpected EOF."),
		);
	}
	let mut fd: std::os::unix::io::RawFd = 0;
	libc::memcpy(
		std::ptr::addr_of_mut!(fd).cast(),
		libc::CMSG_DATA(cmsg).cast(),
		std::mem::size_of_val(&fd),
	);

	if fd > 0 {
		libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
	}

	Ok(std::fs::File::from_raw_fd(fd))
}

async fn unmount(path: &Path) -> crate::Result<()> {
	tokio::process::Command::new("fusermount3")
		.arg("-q")
		.arg("-u")
		.arg(path)
		.status()
		.await
		.wrap_err("Failed to execute the unmount command.")?;
	Ok(())
}
