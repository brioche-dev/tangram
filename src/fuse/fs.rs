use futures::future::try_join_all;
use std::ffi::OsString;
use std::os::unix::prelude::OsStrExt;
use std::path::PathBuf;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{collections::BTreeMap, num::NonZeroU64, str::FromStr, sync::Arc, time::Duration};
use zerocopy::AsBytes;

use crate::template;
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
	artifact: Option<Artifact>, // TODO: use artifact::Hash internally.
	parent: NodeID,
	children: Vec<NodeID>,
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

#[derive(Debug)]
pub struct DirectoryEntry {
	offset: usize,
	node: NodeID,
	name: String,
	kind: FileKind,
}

/// Represents the files we expose through FUSE.
#[derive(Debug)]
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

	/// Look up a filesystem entry from a given parent node and subpath.
	#[tracing::instrument(skip(self), ret)]
	async fn lookup(&self, parent: NodeID, name: &str) -> Result<Entry> {
		// First we need to convert the <parent>/name into an underlying artifact.
		let (node, artifact) = self.lookup_inner(parent, name).await?;

		// Get the artifact metadata.
		let kind = (&artifact).into();
		let size = if let Some(file) = artifact.as_file() {
			let blob = file.blob();
			let bytes = blob.bytes(&self.tg).await.or(Err(libc::EIO))?;
			bytes.len()
		} else {
			0
		};

		let valid_time = self.entry_valid_time();

		let entry = Entry {
			node,
			valid_time,
			kind,
			size,
		};
		Ok(entry)
	}

	async fn lookup_inner(&self, parent: NodeID, name: &str) -> Result<(NodeID, Artifact)> {
		if name == "." {
			let artifact = self.tree()?.artifact(parent)?;
			return Ok((parent, artifact));
		}
		if name == ".." {
			let parent = self.tree()?.parent(parent)?;
			let artifact = self.tree()?.artifact(parent)?;
			return Ok((parent, artifact));
		}

		let (node, artifact) = if parent == ROOT {
			// If the parent is ROOT, we parse the name as a hash and get the artifact.
			let hash = artifact::Hash::from_str(name).or(Err(libc::EINVAL))?;
			let artifact = Artifact::get(&self.tg, hash).await.map_err(|e| {
				tracing::error!(?e);
				libc::EIO
			})?;
			let node = self.tree_mut()?.lookup(ROOT, &artifact);
			(node, artifact)
		} else {
			// Otherwise we get the parent directory (which must already exist) and find the corresponding entry.
			let directory = {
				self.tree()?
					.artifact(parent)?
					.as_directory()
					.ok_or(libc::ENOENT)?
					.to_data()
					.entries
			};

			// Lookup the artifact hash by name.
			let hash = *directory.get(name).ok_or(libc::ENOENT)?;
			let artifact = Artifact::get(&self.tg, hash).await.map_err(|e| {
				tracing::error!(?e);
				libc::EIO
			})?;
			let node = self.tree_mut()?.lookup(parent, &artifact);
			(node, artifact)
		};

		// If the node is None, it means we haven't created one for this entry yet.
		let node = node.unwrap_or(self.tree_mut()?.insert(parent, artifact.clone()));

		Ok((node, artifact))
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

			_ => Err(libc::ENOENT),
		}
	}

	#[tracing::instrument(skip(self), ret)]
	async fn read_link(&self, node: NodeID) -> Result<OsString> {
		// Check that the artifact pointed to by node is actually a symlink.
		let symlink = self
			.tree()?
			.artifact(node)?
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
			let mut parent = self.tree()?.parent(node)?;
			while parent != ROOT {
				result.push("..");
				parent = self.tree()?.parent(parent)?;
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
		let _entry = self.tree()?.artifact(node)?;
		Ok(None)
	}

	/// Read from a regular file.
	#[tracing::instrument(skip(self))]
	async fn read(
		&self,
		node: NodeID,
		_fh: Option<FileHandle>,
		offset: isize,
		length: usize,
		_flags: i32,
	) -> Result<Vec<u8>> {
		let file = {
			self.tree()?
				.artifact(node)?
				.into_file()
				.ok_or(libc::ENOENT)?
		};

		let blob = file.blob().bytes(&self.tg).await.or(Err(libc::EIO))?;
		let start: usize = offset.try_into().or(Err(libc::EINVAL))?;
		let end = (start + length).min(blob.len());
		let contents = blob.get(start..end).ok_or(libc::EINVAL)?;
		Ok(contents.to_owned())
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
			.tree()?
			.artifact(node)?
			.into_directory()
			.ok_or(libc::ENOENT)?;
		Ok(FileHandle::new(0))
	}

	/// Read a directory.
	#[tracing::instrument(skip(self), ret)]
	async fn read_dir(
		&self,
		node: NodeID,
		_fh: Option<FileHandle>,
		_flags: i32,
		_offset: isize,
		_size: usize,
	) -> Result<Vec<DirectoryEntry>> {
		// TODO: track state w/ a file handle and make sure we don't overflow the MAX_WRITE size configured by init.
		let directory = {
			self.tree()?
				.artifact(node)?
				.into_directory()
				.ok_or(libc::ENOENT)?
		};

		let entries = directory.to_data().entries.into_iter().enumerate().map(
			|(offset, (name, _))| async move {
				let (node, artifact) = self.lookup_inner(node, &name).await?;
				let kind = (&artifact).into();

				Ok(DirectoryEntry {
					offset,
					node,
					name,
					kind,
				})
			},
		);

		try_join_all(entries).await
	}

	/// Release a directory.
	#[tracing::instrument(skip(self), ret)]
	async fn release_dir(&self, node: NodeID) -> Result<()> {
		// TODO: release dir
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

	fn tree(&self) -> Result<RwLockReadGuard<'_, FileSystem>> {
		self.tree.read().map_err(|_| libc::EIO)
	}

	fn tree_mut(&self) -> Result<RwLockWriteGuard<'_, FileSystem>> {
		self.tree.write().map_err(|_| libc::EIO)
	}
}

impl FileSystem {
	fn new() -> Self {
		let root = (
			ROOT,
			Node {
				artifact: None,
				parent: ROOT,
				children: Vec::new(),
			},
		);
		Self {
			data: [root].into_iter().collect(),
		}
	}

	fn insert(&mut self, parent: NodeID, artifact: Artifact) -> NodeID {
		let node = NodeID(self.data.len() as u64 + 1000);
		let data = Node {
			artifact: Some(artifact),
			parent,
			children: Vec::new(),
		};
		self.data.insert(node, data);

		if let Some(parent_data) = self.data.get_mut(&parent) {
			parent_data.children.push(node);
		}

		node
	}

	fn lookup(&mut self, parent: NodeID, artifact: &Artifact) -> Option<NodeID> {
		self.data
			.get(&parent)
			.and_then(|p| {
				p.children.iter().find(|node| {
					let existing = self
						.data
						.get(node)
						.and_then(|data| data.artifact.as_ref().map(Artifact::hash));

					existing == Some(artifact.hash())
				})
			})
			.copied()
	}

	fn artifact(&self, node: NodeID) -> Result<Artifact> {
		self.data
			.get(&node)
			.and_then(|data| data.artifact.clone())
			.ok_or(libc::ENOENT)
	}

	fn parent(&self, node: NodeID) -> Result<NodeID> {
		let data = self.data.get(&node).ok_or(libc::ENOENT)?;
		Ok(data.parent)
	}

	fn _add_ref(&mut self, _node: NodeID) {
		// TODO: add_ref
	}

	fn _release(&mut self, _node: NodeID) {
		// TODO: release
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

impl From<Vec<DirectoryEntry>> for Response {
	fn from(value: Vec<DirectoryEntry>) -> Self {
		let mut buf = Vec::new();
		for entry in value {
			let ino = entry.node.0;
			let name = entry.name.as_bytes();
			let namelen = name.len().try_into().expect("Name too long.");
			let off = entry.offset.try_into().expect("Offset too larger.");
			let typ = entry.kind.type_();
			let header = abi::fuse_dirent {
				ino,
				off,
				namelen,
				typ,
			};
			buf.extend_from_slice(header.as_bytes());
			buf.extend_from_slice(name);
		}

		Response::Data(buf)
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
