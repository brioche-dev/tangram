use super::sys;
use crate::{
	artifact::Artifact,
	blob,
	directory::Directory,
	error::{Error, Result, WrapErr},
	file::File,
	instance::Instance,
	symlink::Symlink,
	template,
};
use num_traits::ToPrimitive;
use std::{
	collections::BTreeMap,
	ffi::CString,
	io::{SeekFrom, Write},
	sync::{Arc, Weak},
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use zerocopy::{AsBytes, FromBytes};

/// A FUSE server.
#[derive(Clone)]
pub struct Server {
	tg: Instance,
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
	pub fn new(tg: Instance) -> Self {
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
		Self { tg, state }
	}

	/// Serve.
	#[allow(clippy::too_many_lines)]
	pub async fn serve(self, mut fuse_file: tokio::fs::File) -> Result<()> {
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
					_ => return Err(error.into()),
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
						.map_err(Error::other)
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
			let mut fuse_file = fuse_file.try_clone().await?.into_std().await;
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
		let response = node.fuse_attr_out();
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
		let response = child_node.fuse_entry_out();

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
			NodeKind::Root { .. } => return Err(libc::EIO),
			NodeKind::Directory { .. } => FileHandleData::Directory,
			NodeKind::File { file, .. } => {
				let Ok(reader) = file.reader(&self.tg).await else {
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
		for (offset, name) in [".", ".."]
			.into_iter()
			.chain(directory.names())
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
					entry_out: node.fuse_entry_out(),
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
		let target = match &node.kind {
			NodeKind::Symlink { symlink, .. } => symlink.target().clone(),
			_ => return Err(libc::EIO),
		};

		// Render the target.
		let mut response = String::new();
		for component in target.components() {
			use std::fmt::Write;
			match component {
				template::Component::String(string) => {
					write!(&mut response, "{string}").unwrap();
				},
				template::Component::Artifact(artifact) => {
					let id = artifact.id(&self.tg).await?;
					write!(&mut response, "/.tangram/artifacts/{id}").unwrap();
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

			NodeKind::Directory { directory, .. } => directory
				.try_get_entry(&self.tg, name)
				.await
				.map_err(|_| libc::EIO)?
				.ok_or(libc::EIO)?,

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
				let size = file.size(&self.tg).await.map_err(|_| libc::EIO)?;
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
			NodeKind::Root { .. } | NodeKind::Directory { .. } => libc::S_IFDIR,
			NodeKind::File { .. } => libc::S_IFREG,
			NodeKind::Symlink { .. } => libc::S_IFLNK,
		}
	}

	fn mode(&self) -> u32 {
		match &self.kind {
			NodeKind::Root { .. } | NodeKind::Directory { .. } => libc::S_IFDIR | 0o555,
			NodeKind::File { file, .. } => {
				libc::S_IFREG | 0o444 | (if file.executable() { 0o111 } else { 0o000 })
			},
			NodeKind::Symlink { .. } => libc::S_IFLNK | 0o444,
		}
	}

	fn size(&self) -> u64 {
		match &self.kind {
			NodeKind::Root { .. } | NodeKind::Directory { .. } | NodeKind::Symlink { .. } => 0,
			NodeKind::File { size, .. } => *size,
		}
	}

	fn fuse_entry_out(&self) -> sys::fuse_entry_out {
		let nodeid = self.id.0;
		let attr_out = self.fuse_attr_out();
		sys::fuse_entry_out {
			nodeid,
			generation: 0,
			entry_valid: 1024,
			attr_valid: 0,
			entry_valid_nsec: 1024,
			attr_valid_nsec: 0,
			attr: attr_out.attr,
		}
	}

	fn fuse_attr_out(&self) -> sys::fuse_attr_out {
		let nodeid = self.id.0;
		let nlink = match &self.kind {
			NodeKind::Root { .. } => 2,
			_ => 1,
		};
		let size = self.size();
		let mode = self.mode();
		sys::fuse_attr_out {
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
		}
	}
}
