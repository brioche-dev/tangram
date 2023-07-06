use std::ffi::OsString;
use std::io::SeekFrom;
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};
// use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{collections::BTreeMap, num::NonZeroU64, str::FromStr, sync::Arc, time::Duration};
use tokio::{
	io::{AsyncReadExt, AsyncSeekExt},
	sync::RwLock,
};
use zerocopy::AsBytes;

use crate::template::{self};
use crate::{
	artifact::{self, Artifact},
	instance::Instance,
};

use super::{
	abi,
	request::{Arg, Request},
	response::Response,
};

/// All filesystem and server methods need to return an error code, using the standard values for errno from libc.
type Result<T> = std::result::Result<T, i32>;

/// The FUSE implementation.
#[derive(Clone)]
pub struct Server {
	tg: Arc<Instance>,
	tree: Arc<RwLock<FileSystem>>,
}

/// The underlying file system implementation.
#[derive(Default)]
struct FileSystem {
	data: BTreeMap<NodeID, Node>,
}

/// A single node in the file system.
struct Node {
	name: Option<String>,
	hash: Option<artifact::Hash>,
	kind: FileKind,
	parent: NodeID,
	children: Option<Vec<NodeID>>,
}

/// A node in the file system.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct NodeID(u64);

/// The root node has a reserved ID of 1.
const ROOT: NodeID = NodeID(1);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct FileHandle(NonZeroU64);

#[derive(Debug)]
struct Entry {
	node: NodeID,
	valid_time: Duration,
	size: usize,
	kind: FileKind,
}

#[derive(Debug)]
pub struct Attr {
	node: NodeID,
	valid_time: Duration,
	kind: FileKind,
	size: usize,
	num_hardlinks: u32,
}

/// Represents the files we expose through FUSE.
#[derive(Debug, Copy, Clone)]
pub enum FileKind {
	Directory,
	File { is_executable: bool },
	Symlink,
}

impl Server {
	/// Create a new file system instance.
	pub fn new(tg: Arc<Instance>) -> Self {
		Self {
			tg,
			tree: Arc::new(RwLock::new(FileSystem::new())),
		}
	}

	/// Service a file system request from the FUSE server.
	pub async fn handle_request(&self, request: Request) -> Response {
		let node = NodeID(request.header.nodeid);

		match &request.arg {
			Arg::GetAttr => self.get_attr(node).await.into(),
			Arg::Lookup(arg) => match arg.to_str() {
				None => {
					tracing::error!(?arg, "Failed to parse path as UTF-8.");
					Response::error(libc::EINVAL)
				},
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
			Arg::Unsupported(opcode) => {
				// Processes will call ioctl() in order to determine if a device is a TTY or regular file.
				if *opcode == abi::fuse_opcode::FUSE_IOCTL {
					Response::error(libc::ENOTTY)
				} else {
					tracing::error!(?opcode, "Unsupported FUSE request.");
					Response::error(libc::ENOSYS)
				}
			},
			Arg::Initialize(_) | Arg::Destroy => unreachable!(),
		}
	}

	/// Look up a filesystem entry from a given parent node and subpath.
	#[tracing::instrument(skip(self), ret)]
	async fn lookup(&self, parent: NodeID, name: &str) -> Result<Entry> {
		// Make sure the directory entries have been cached already.
		if parent != ROOT {
			self.ensure_directory_is_cached(parent).await?;
		}

		// First we need to convert the <parent>/name into an underlying artifact.
		let (node, kind) = self.lookup_inner(parent, name).await?;

		// Get the artifact metadata.
		let size = self.size(node).await?;
		let valid_time = self.entry_valid_time();

		let entry = Entry {
			node,
			valid_time,
			kind,
			size,
		};

		Ok(entry)
	}

	/// Lookup an entry in the file system by name.
	async fn lookup_inner(&self, parent: NodeID, name: &str) -> Result<(NodeID, FileKind)> {
		// Handle special cases, "." and "..".
		if name == "." {
			return Ok((parent, FileKind::Directory));
		}
		if name == ".." {
			let parent = self.tree.read().await.parent(parent)?;
			return Ok((parent, FileKind::Directory));
		}

		// If the parent node isn't ROOT we can do a simple lookup in the tree.
		if parent != ROOT {
			return self
				.tree
				.write()
				.await
				.lookup(parent, name)
				.map(|(node, data)| (node, data.kind))
				.ok_or(libc::ENOENT);
		}

		// Check if the artifact has already been added at the root.
		let result = self
			.tree
			.read()
			.await
			.lookup(ROOT, name)
			.map(|(node, data)| (node, data.kind));

		if let Some((node, kind)) = result {
			Ok((node, kind))
		} else {
			// Otherwise, get the artifact and insert it into the file system.
			let hash = artifact::Hash::from_str(name).map_err(|e| {
				tracing::error!(?name, ?e, "Failed to parse path as an artifact hash.");
				libc::EINVAL
			})?;
			let artifact = Artifact::get(&self.tg, hash).await.map_err(|e| {
				tracing::error!(?e, "Failed to get artifact at root.");
				libc::EIO
			})?;
			let kind = (&artifact).into();
			let node = self
				.tree
				.write()
				.await
				.insert(ROOT, name.to_owned(), artifact)?;
			Ok((node, kind))
		}
	}

	/// Get file system attributes.
	#[tracing::instrument(skip(self), ret)]
	async fn get_attr(&self, node: NodeID) -> Result<Attr> {
		match node.0 {
			1 => Ok(Attr {
				node,
				valid_time: self.attr_valid_time(),
				kind: FileKind::Directory,
				num_hardlinks: 2,
				size: 0,
			}),
			_ => {
				let artifact = self.get_artifact(node).await?;
				let size = self.size(node).await?;

				Ok(Attr {
					node,
					valid_time: self.attr_valid_time(),
					kind: (&artifact).into(),
					num_hardlinks: 1,
					size,
				})
			},
		}
	}

	#[tracing::instrument(skip(self), ret)]
	async fn read_link(&self, node: NodeID) -> Result<OsString> {
		// Check that the artifact pointed to by node is actually a symlink.
		let symlink = self
			.get_artifact(node)
			.await?
			.into_symlink()
			.ok_or(libc::EINVAL)?;

		// Grab the target and attempt to parse it into the [artifact] [subpath...]
		let target = symlink.target();
		let mut artifact = None;
		let mut subpath = Vec::new();
		for (i, component) in target.components().iter().enumerate() {
			match component {
				template::Component::Artifact(_) if i != 0 => {
					tracing::error!(?target, "Invalid symlink target.");
					return Err(libc::EINVAL);
				},
				template::Component::Artifact(a) => {
					artifact = Some(a.clone());
				},
				template::Component::Placeholder(placeholder) => {
					tracing::error!(?placeholder, "Cannot resolve placeholders in symlinks.");
					return Err(libc::EINVAL);
				},
				template::Component::String(string) => {
					// TODO: use std::path here.
					subpath.extend(string.split('/'));
				},
			}
		}

		let mut result = PathBuf::new();
		if let Some(artifact) = artifact {
			let mut parent = self.tree.read().await.parent(node)?;
			while parent != ROOT {
				result.push("..");
				parent = self.tree.read().await.parent(parent)?;
			}
			result.push(artifact.hash().to_string());
		}

		for component in subpath {
			result.push(component);
		}

		Ok(result.as_os_str().to_owned())
	}

	/// Open a regular file.
	#[tracing::instrument(skip(self), ret)]
	async fn open(&self, node: NodeID, flags: i32) -> Result<Option<FileHandle>> {
		let _entry = self.get_artifact(node).await?;
		self.tree.write().await.add_ref(node)?;
		Ok(None)
	}

	/// Read from a regular file.
	#[tracing::instrument(skip(self, _fh, _flags))]
	async fn read(
		&self,
		node: NodeID,
		_fh: Option<FileHandle>,
		offset: isize,
		length: usize,
		_flags: i32,
	) -> Result<Vec<u8>> {
		let file = self.get_artifact(node).await?.into_file().ok_or_else(|| {
			tracing::error!(?node, "Failed to get file artifact.");
			libc::ENOENT
		})?;

		let blob = file.blob();
		let mut reader = blob
			.try_get_local(&self.tg)
			.await
			.map_err(|e| {
				tracing::error!(?e, ?node, "Failed to get the underlying blob.");
				libc::EIO
			})?
			.ok_or_else(|| {
				tracing::error!(?node, "Failed to get reader for the blob.");
				libc::EIO
			})?;

		// Get the start and end positions of the stream.
		let start = reader
			.seek(SeekFrom::Start(offset.try_into().unwrap()))
			.await
			.map_err(|e| {
				tracing::error!(?e, "Failed to seek to start of file.");
				e.raw_os_error().unwrap_or(libc::EIO)
			})?;

		let end = reader.seek(SeekFrom::End(0)).await.map_err(|e| {
			tracing::error!(?e, "Failed to seek to end of file.");
			e.raw_os_error().unwrap_or(libc::EIO)
		})?;

		// Seek back to the offset.
		reader
			.seek(SeekFrom::Start(offset.try_into().unwrap()))
			.await
			.map_err(|e| {
				tracing::error!(?e);
				e.raw_os_error().unwrap_or(libc::EIO)
			})?;

		// Read the contents from the stream.
		let mut buf = Vec::new();
		buf.resize_with(length.min((end - start).try_into().unwrap()), || 0);
		reader.read_exact(&mut buf).await.map_err(|e| {
			tracing::error!(?e, "Failed to read.");
			e.raw_os_error().unwrap_or(libc::EIO)
		})?;

		Ok(buf)
	}

	/// Release a regular file. Note: potentially called many times for the same node.
	#[tracing::instrument(skip(self), ret)]
	async fn release(&self, node: NodeID) -> Result<()> {
		// TODO: implement release
		Ok(())
	}

	/// Open a directory. TODO:make sure we return a real file handle.
	#[tracing::instrument(skip(self), ret)]
	async fn open_dir(&self, node: NodeID, _flags: i32) -> Result<Option<FileHandle>> {
		let _entry = self
			.get_artifact(node)
			.await?
			.into_directory()
			.ok_or_else(|| {
				tracing::error!(?node, "Failed to get artifact as directory.");
				libc::ENOENT
			})?;
		self.tree.write().await.add_ref(node)?;
		Ok(FileHandle::new(0))
	}

	/// Read a directory.
	///
	/// Since our server does not support adding, removing, or renaming directory entries we do not have to worry about concurrent modifications to the underlying directory object, and as a result we do not need to track directory stream state using the file handle or offset value.
	#[tracing::instrument(skip(self, _fh, _flags, size))]
	async fn read_dir(
		&self,
		node: NodeID,
		_fh: Option<FileHandle>,
		_flags: i32,
		offset: isize,
		size: usize,
	) -> Result<Response> {
		if node != ROOT {
			self.ensure_directory_is_cached(node).await?;
		}

		let tree = self.tree.read().await;
		let entries = tree
			.children(node)?
			.iter()
			.skip(offset as usize)
			.filter_map(|node| {
				let data = tree.data.get(node)?;
				Some((*node, data.name.as_deref().unwrap(), data.kind))
			});

		let mut buf = Vec::with_capacity(size);
		for (n, (node, name, kind)) in entries.enumerate() {
			// TODO: Do we need to convert to an OsStr here?
			let path: &Path = name.as_ref();
			let name_bytes = path.as_os_str().as_bytes();
			let offset = 1 + (n as isize + offset) as i64;
			let dirent = abi::fuse_dirent {
				ino: node.0,
				off: offset,
				namelen: name_bytes.len().try_into().unwrap(),
				typ: kind.type_(),
			};

			let dirent_bytes = dirent.as_bytes();
			let new_size = dirent_bytes.len() + name_bytes.len() + buf.len();
			if new_size > buf.capacity() {
				break;
			}
			buf.extend_from_slice(dirent_bytes);
			buf.extend_from_slice(name_bytes);
		}
		Ok(Response::Data(buf))
	}

	/// Release a directory.
	#[tracing::instrument(skip(self), ret)]
	async fn release_dir(&self, node: NodeID) -> Result<()> {
		self.tree.write().await.release(node)?;
		Ok(())
	}

	/// Flush calls to the file. Happens after open and before release.
	#[tracing::instrument(skip(self), ret)]
	async fn flush(&self, _node: NodeID, _fh: Option<FileHandle>) -> Result<()> {
		Ok(())
	}

	fn attr_valid_time(&self) -> Duration {
		Duration::from_micros(100)
	}

	fn entry_valid_time(&self) -> Duration {
		self.attr_valid_time()
	}

	async fn size(&self, node: NodeID) -> Result<usize> {
		let artifact = self.get_artifact(node).await?;
		if let Artifact::File(file) = artifact {
			let blob = file.blob();
			let mut reader = blob
				.try_get_local(&self.tg)
				.await
				.map_err(|e| {
					tracing::error!(?e, ?node, "Failed to get the underlying blob.");
					libc::EIO
				})?
				.ok_or_else(|| {
					tracing::error!(?node, "Failed to get reader for the blob.");
					libc::EIO
				})?;

			reader.seek(SeekFrom::Start(0)).await.map_err(|e| {
				tracing::error!(?e, "Failed to seek to beginning of file.");
				e.raw_os_error().unwrap_or(libc::EIO)
			})?;

			let end = reader.seek(SeekFrom::End(0)).await.map_err(|e| {
				tracing::error!(?e, "Failed to seek to end of file.");
				e.raw_os_error().unwrap_or(libc::EIO)
			})?;

			Ok(end.try_into().unwrap())
		} else {
			Ok(0)
		}
	}

	async fn get_artifact(&self, node: NodeID) -> Result<Artifact> {
		let hash = self.tree.read().await.hash(node)?;
		Artifact::get(&self.tg, hash).await.map_err(|e| {
			tracing::error!(?e, ?hash, "Failed to get artifact.");
			libc::EIO
		})
	}

	/// Eagerly fetch the children of the directory and insert them to the file system for fast subsequent lookups.
	async fn ensure_directory_is_cached(&self, node: NodeID) -> Result<()> {
		let mut tree = self.tree.write().await;
		let data = tree.data.get_mut(&node).ok_or_else(|| {
			tracing::error!(?node, "Failed to get node as a directory.");
			libc::EIO
		})?;

		if data.children.is_some() {
			return Ok(());
		}

		let artifact = Artifact::get(&self.tg, data.hash.unwrap())
			.await
			.map_err(|e| {
				tracing::error!(?e, "Failed to get the artifact.");
				libc::EIO
			})?
			.into_directory()
			.ok_or_else(|| {
				tracing::error!("Failed to get artifact as a directory.");
				libc::EIO
			})?;

		let entries = artifact.entries(&self.tg).await.map_err(|e| {
			tracing::error!(?e, "Failed to get directory entries.");
			libc::EIO
		})?;

		data.children = Some(Vec::new());
		for (name, artifact) in entries {
			tree.insert(node, name, artifact)?;
		}

		Ok(())
	}
}

impl FileSystem {
	fn new() -> Self {
		let root = (
			ROOT,
			Node {
				name: None,
				hash: None,
				kind: FileKind::Directory,
				parent: ROOT,
				children: Some(Vec::new()),
			},
		);

		Self {
			data: [root].into_iter().collect(),
		}
	}

	fn insert(&mut self, parent: NodeID, name: String, artifact: Artifact) -> Result<NodeID> {
		let node = NodeID(self.data.len() as u64 + 1000);
		let data = Node {
			name: Some(name),
			hash: Some(artifact.hash()),
			kind: (&artifact).into(),
			parent,
			children: None,
		};
		self.data.insert(node, data);

		// Insert into the parent.
		self.data
			.get_mut(&parent)
			.unwrap()
			.children
			.as_mut()
			.ok_or_else(|| {
				tracing::error!("Failed to insert child into directory.");
				libc::EIO
			})?
			.push(node);

		Ok(node)
	}

	fn lookup(&self, parent: NodeID, name: &str) -> Option<(NodeID, &'_ Node)> {
		let parent = self.data.get(&parent)?;
		let children = parent.children.as_ref()?;
		children.iter().find_map(|node| {
			let data = self.data.get(node)?;
			if data.name.as_deref() == Some(name) {
				Some((*node, data))
			} else {
				None
			}
		})
	}

	fn hash(&self, node: NodeID) -> Result<artifact::Hash> {
		self.data
			.get(&node)
			.and_then(|data| data.hash)
			.ok_or_else(|| {
				tracing::error!(?node, "Failed to retrieve artifact hash.");
				libc::ENOENT
			})
	}

	fn parent(&self, node: NodeID) -> Result<NodeID> {
		let data = self.data.get(&node).ok_or_else(|| {
			tracing::error!(?node, "Failed to retrieve parent.");
			libc::ENOENT
		})?;
		Ok(data.parent)
	}

	fn children(&self, node: NodeID) -> Result<&'_ [NodeID]> {
		self.data
			.get(&node)
			.ok_or_else(|| {
				tracing::error!(?node, "Node does not exist.");
				libc::EIO
			})?
			.children
			.as_deref()
			.ok_or_else(|| {
				tracing::error!(?node, "Failed to get children of a node.");
				libc::EIO
			})
	}

	fn add_ref(&mut self, _node: NodeID) -> Result<()> {
		// TODO
		Ok(())
	}

	fn release(&mut self, _node: NodeID) -> Result<()> {
		// TODO
		Ok(())
	}
}

impl FileHandle {
	fn new(fh: u64) -> Option<FileHandle> {
		Some(FileHandle(NonZeroU64::new(fh)?))
	}
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

impl From<OsString> for Response {
	fn from(value: OsString) -> Self {
		Response::Data(value.as_bytes().to_owned())
	}
}

impl From<Option<FileHandle>> for Response {
	fn from(value: Option<FileHandle>) -> Self {
		let response = abi::fuse_open_out {
			fh: value.map_or(0, |fh| fh.0.get()),
			open_flags: 0,
			padding: 0,
		};
		Response::data(response.as_bytes())
	}
}

impl<'a> From<&'a Artifact> for FileKind {
	fn from(value: &'a Artifact) -> Self {
		match value {
			Artifact::File(f) => FileKind::File {
				is_executable: f.executable(),
			},
			Artifact::Directory(_) => FileKind::Directory,
			Artifact::Symlink(_) => FileKind::Symlink,
		}
	}
}
