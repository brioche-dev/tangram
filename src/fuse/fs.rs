use std::{collections::BTreeMap, num::NonZeroU64, str::FromStr, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use zerocopy::AsBytes;

use crate::{
	artifact::{self, Artifact},
	directory::Directory,
	file::File,
	instance::Instance,
};

use super::{
	abi,
	request::{Arg, Request},
	response::Response,
};
type Result<T> = std::result::Result<T, i32>;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct NodeID(u64);
const ROOT: NodeID = NodeID(1);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct FileHandle(NonZeroU64);

impl FileHandle {
	fn new(fh: u64) -> Option<FileHandle> {
		Some(FileHandle(NonZeroU64::new(fh)?))
	}
}

#[derive(Clone)]
pub struct FileSystem {
	tg: Arc<Instance>,
	state: Arc<Mutex<State>>,
}

#[derive(Default)]
pub struct State {
	root: BTreeMap<artifact::Hash, NodeID>,
	nodes: BTreeMap<NodeID, artifact::Hash>,
}

impl State {
	fn add_node(&mut self, hash: artifact::Hash) -> NodeID {
		let node = NodeID(self.nodes.len() as u64 + 1000);
		let _ = self.nodes.insert(node, hash);
		node
	}

	fn add_to_root(&mut self, hash: artifact::Hash) -> NodeID {
		if let Some(node) = self.root.get(&hash) {
			*node
		} else {
			let node = self.add_node(hash);
			let _ = self.root.insert(hash, node);
			node
		}
	}

	async fn artifact(&self, node: NodeID, tg: &Instance) -> Result<artifact::Artifact> {
		let hash = self.nodes.get(&node).ok_or(libc::ENOENT)?;

		Artifact::get(tg, *hash).await.or(Err(libc::EIO))
	}

	async fn directory(&self, node: NodeID, tg: &Instance) -> Result<Directory> {
		self.artifact(node, tg)
			.await
			.and_then(|a| a.into_directory().ok_or(libc::ENOENT))
	}

	async fn file(&self, node: NodeID, tg: &Instance) -> Result<File> {
		self.artifact(node, tg)
			.await
			.and_then(|a| a.into_file().ok_or(libc::ENOENT))
	}

	async fn kind(&self, node: NodeID, tg: &Instance) -> Result<FileKind> {
		let artifact = self.artifact(node, tg).await?;
		let kind = match artifact {
			Artifact::File(file) => FileKind::File {
				is_executable: file.executable(),
			},
			Artifact::Symlink(_) => FileKind::Symlink,
			Artifact::Directory(_) => FileKind::Directory,
		};

		Ok(kind)
	}

	async fn size(&self, node: NodeID, tg: &Instance) -> Result<usize> {
		let artifact = self.artifact(node, tg).await?;
		match artifact {
			Artifact::File(file) => {
				let bytes = file.blob().bytes(tg).await.or(Err(libc::EIO))?;
				Ok(bytes.len())
			},
			_ => Ok(0),
		}
	}
}

#[derive(Debug)]
struct FileData {
	size: usize,
	kind: FileKind,
}

/// Represents the files we expose through FUSE.

#[derive(Debug)]
pub enum FileKind {
	Directory,
	File { is_executable: bool },
	Symlink,
}

impl FileKind {
	fn type_(&self) -> u32 {
		match self {
			Self::Directory => libc::S_IFDIR,
			Self::File { is_executable: _ } => libc::S_IFREG,
			Self::Symlink => libc::S_IFLNK,
		}
	}

	fn permissions(&self) -> u32 {
		match self {
			Self::Directory => libc::S_IEXEC | libc::S_IREAD,
			Self::File { is_executable } if *is_executable => libc::S_IEXEC | libc::S_IREAD,
			_ => libc::S_IREAD,
		}
	}

	fn mode(&self) -> u32 {
		self.type_() | self.permissions()
	}
}

#[derive(Debug)]
pub struct Entry {
	pub node: NodeID,
	pub valid_time: Duration,
	pub size: usize,
	pub kind: FileKind,
}

#[derive(Debug)]
pub struct Attr {
	pub node: NodeID,
	pub valid_time: Duration,
	pub kind: FileKind,
	pub size: usize,
	pub num_hardlinks: usize,
}

#[derive(Debug)]
pub struct DirectoryEntry {
	pub offset: usize,
	pub node: NodeID,
	pub name: String,
	pub kind: FileKind,
}

impl FileSystem {
	pub fn new(tg: Arc<Instance>) -> Self {
		Self {
			tg,
			state: Arc::new(Mutex::new(State::default())),
		}
	}

	pub async fn handle_request(&self, request: Request) -> Response {
		let node = NodeID(request.header.nodeid);

		match &request.arg {
			Arg::GetAttr => self.get_attr(node).await.into(),
			Arg::Lookup(arg) => match arg.to_str() {
				None => Response::error(libc::EINVAL),
				Some(name) => self.lookup(node, name).await.into(),
			},
			Arg::Open(arg) => self.open(node, arg.flags).await.into(),
			Arg::OpenDir(arg) => self.open_dir(node, arg.flags).await.into(),
			Arg::Read(arg) => self
				.read(
					node,
					FileHandle::new(arg.fh),
					arg.offset as isize,
					arg.size as usize,
					arg.flags,
				)
				.await
				.into(),
			Arg::ReadDir(arg) => self
				.read_dir(
					node,
					FileHandle::new(arg.fh),
					arg.flags,
					arg.offset as isize,
					arg.size as usize,
				)
				.await
				.into(),
			Arg::ReadLink => self.read_link(node).await.into(),
			Arg::Flush(arg) => self.flush(node, FileHandle::new(arg.fh)).await.into(),
			Arg::Release => self.release(node).await.into(),
			Arg::ReleaseDir => self.release_dir(node).await.into(),
			Arg::Unknown => Response::error(libc::ENOSYS),
			Arg::Initialize(_) | Arg::Destroy => unreachable!(),
		}
	}

	#[tracing::instrument(skip(self), ret)]
	pub async fn lookup(&self, parent: NodeID, name: &str) -> Result<Entry> {
		// Acquire a lock on the state.
		let mut state = self.state.lock().await;

		// Get the NodeID corresponding to <parent>/name, creating one if it does not exist.
		let node = if parent == ROOT {
			let hash = artifact::Hash::from_str(name).or(Err(libc::EINVAL))?;
			state.add_to_root(hash)
		} else {
			let parent = state.directory(parent, &*self.tg).await?.to_data();
			let hash = *parent.entries.get(name).ok_or(libc::ENOENT)?;
			state.add_node(hash)
		};

		// Get the artifact metadata.
		let kind = state.kind(node, &self.tg).await?;
		let size = state.size(node, &self.tg).await?;
		let valid_time = self.entry_valid_time();

		let entry = Entry {
			node,
			kind,
			valid_time,
			size,
		};
		Ok(entry)
	}

	#[tracing::instrument(skip(self), ret)]
	pub async fn get_attr(&self, node: NodeID) -> Result<Attr> {
		match node.0 {
			1 => Ok(Attr {
				node,
				valid_time: self.attr_valid_time(),
				kind: FileKind::Directory,
				num_hardlinks: 2,
				size: 0,
			}),

			_ => Err(libc::ENOENT),
		}
	}

	#[tracing::instrument(skip(self), ret)]
	pub async fn read_link(&self, _node: NodeID) -> Result<Entry> {
		Err(libc::ENOSYS)
	}

	#[tracing::instrument(skip(self), ret)]
	pub async fn open(&self, node: NodeID, flags: i32) -> Result<Option<FileHandle>> {
		let _file = self.state.lock().await.file(node, &*self.tg).await?;
		// TODO: create a file handle
		Ok(None)
	}

	#[tracing::instrument(skip(self), ret)]
	pub async fn read(
		&self,
		node: NodeID,
		_fh: Option<FileHandle>,
		offset: isize,
		length: usize,
		_flags: i32,
	) -> Result<Vec<u8>> {
		let state = self.state.lock().await;
		let file = state.file(node, &*self.tg).await?;
		let blob = file.blob().bytes(&self.tg).await.or(Err(libc::EIO))?;
		let range = (offset as usize)..(length.min(blob.len()));
		Ok(blob[range].to_owned())
	}

	#[tracing::instrument(skip(self), ret)]
	pub async fn release(&self, _node: NodeID) -> Result<()> {
		Ok(())
	}

	#[tracing::instrument(skip(self), ret)]
	pub async fn open_dir(&self, node: NodeID, flags: i32) -> Result<Option<FileHandle>> {
		let state = self.state.lock().await;
		let _ = state.directory(node, &self.tg);
		Ok(FileHandle::new(0))
	}

	#[tracing::instrument(skip(self), ret)]
	pub async fn read_dir(
		&self,
		node: NodeID,
		_fh: Option<FileHandle>,
		_flags: i32,
		_offset: isize,
		_size: usize,
	) -> Result<Vec<DirectoryEntry>> {
		if node.0 != 1 {
			return Err(libc::ENOENT);
		}

		Ok(vec![
			DirectoryEntry {
				offset: 0,
				node: NodeID(1),
				name: ".".to_owned(),
				kind: FileKind::Directory,
			},
			DirectoryEntry {
				offset: 1,
				node: NodeID(1),
				name: "..".to_owned(),
				kind: FileKind::Directory,
			},
			DirectoryEntry {
				offset: 2,
				node: NodeID(2),
				name: "file.txt".to_owned(),
				kind: FileKind::File {
					is_executable: false,
				},
			},
		])
	}

	#[tracing::instrument(skip(self), ret)]
	pub async fn release_dir(&self, _node: NodeID) -> Result<()> {
		Ok(())
	}

	#[tracing::instrument(skip(self), ret)]
	pub async fn flush(&self, _node: NodeID, _fh: Option<FileHandle>) -> Result<()> {
		Ok(())
	}

	fn attr_valid_time(&self) -> Duration {
		Duration::from_micros(100)
	}

	fn entry_valid_time(&self) -> Duration {
		self.attr_valid_time()
	}

	// TODO: access control.
	fn can_open(&self, _node: NodeID, _flags: i32) -> bool {
		true
	}
}

impl From<()> for Response {
	fn from(_: ()) -> Self {
		Self::error(0)
	}
}

impl<T> From<Result<T>> for Response
where
	T: Into<Response>,
{
	fn from(value: Result<T>) -> Self {
		match value {
			Ok(value) => value.into(),
			Err(err) => Response::error(err),
		}
	}
}

impl From<Attr> for Response {
	fn from(value: Attr) -> Self {
		let time = value.valid_time.as_secs();
		let timensec = value.valid_time.subsec_nanos();

		let response = abi::fuse_attr_out {
			attr_valid: time,
			attr_valid_nsec: timensec,
			dummy: 0,
			attr: abi::fuse_attr {
				ino: value.node.0,
				size: value.size as u64,
				atime: 0,
				ctime: 0,
				mtime: 0,
				atimensec: 0,
				ctimensec: 0,
				mtimensec: 0,
				nlink: value.num_hardlinks as _,
				mode: value.kind.mode(),
				uid: 1000,
				gid: 1000,
				rdev: 0,
				blocks: 0,
				blksize: 512,
				padding: 0,
			},
		};

		Response::data(response.as_bytes())
	}
}

impl From<Entry> for Response {
	fn from(value: Entry) -> Self {
		let time = value.valid_time.as_secs();
		let timensec = value.valid_time.subsec_nanos();

		let response = abi::fuse_entry_out {
			nodeid: value.node.0,
			generation: 0,
			entry_valid: time,
			entry_valid_nsec: timensec,
			attr_valid: time,
			attr_valid_nsec: timensec,
			attr: abi::fuse_attr {
				ino: value.node.0,
				size: value.size as u64,
				atime: 0,
				ctime: 0,
				mtime: 0,
				atimensec: 0,
				ctimensec: 0,
				mtimensec: 0,
				nlink: 1,
				mode: value.kind.mode(),
				uid: 1000,
				gid: 1000,
				rdev: 0,
				blocks: 0,
				blksize: 512,
				padding: 0,
			},
		};

		Response::data(response.as_bytes())
	}
}

impl From<Vec<u8>> for Response {
	fn from(value: Vec<u8>) -> Self {
		Response::Data(value)
	}
}

impl From<Option<FileHandle>> for Response {
	fn from(value: Option<FileHandle>) -> Self {
		let response = abi::fuse_open_out {
			fh: value.map(|fh| fh.0.get()).unwrap_or(0),
			open_flags: 0,
			padding: 0,
		};
		Response::data(response.as_bytes())
	}
}

impl From<Vec<DirectoryEntry>> for Response {
	fn from(value: Vec<DirectoryEntry>) -> Self {
		let mut buf = Vec::new();

		for entry in value {
			let name = entry.name.as_bytes();
			let header = abi::fuse_dirent {
				ino: entry.node.0,
				off: entry.offset as i64,
				namelen: name.len() as u32,
				typ: entry.kind.type_(),
			};
			buf.extend_from_slice(header.as_bytes());
			buf.extend_from_slice(name);
		}

		Response::Data(buf)
	}
}
