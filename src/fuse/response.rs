use std::io::{IoSlice, Write};
use zerocopy::AsBytes;

use super::abi;
use crate::error::Result;

/// A response a a FUSE request.
#[derive(Debug)]
pub enum Response {
	/// An error, containing an errno value.
	Error(i32),

	/// A serialized piece of data to be written to the kernel.
	Data(Vec<u8>),
}

impl Response {
	/// Create a new error response.
	pub fn error(ec: i32) -> Self {
		Self::Error(ec)
	}

	/// Create a response from serialized data.
	pub fn data(data: &[u8]) -> Self {
		Self::Data(data.iter().copied().collect())
	}

	/// Create a response containing a single file system entry.
	pub fn entry(inode: u64, file_size: usize, num_blocks: usize, kind: FileKind) -> Self {
		// TODO: Do these need to be different?
		let attr_ttl = std::time::Duration::from_micros(100);
		let entry_ttl = std::time::Duration::from_micros(100);

		let response = abi::fuse_entry_out {
			nodeid: inode,
			generation: 0,
			entry_valid: entry_ttl.as_secs(),
			entry_valid_nsec: entry_ttl.subsec_nanos(),
			attr_valid: attr_ttl.as_secs(),
			attr_valid_nsec: attr_ttl.subsec_nanos(),
			attr: abi::fuse_attr {
				ino: inode,
				size: file_size as u64,
				blocks: num_blocks as u64,
				atime: 0,
				mtime: 0,
				ctime: 0,
				atimensec: 0,
				mtimensec: 0,
				ctimensec: 0,
				mode: kind.mode(),
				nlink: 1,
				uid: 1000,
				gid: 1000,
				rdev: 0,
				blksize: 512,
				padding: 0,
			},
		};

		Response::data(response.as_bytes())
	}

	/// Create a response containing a list of directory entries.
	pub fn directory(entries: &[DirectoryEntry]) -> Self {
		let mut buf = Vec::new();

		for (i, entry) in entries.iter().enumerate() {
			let name = entry.name.as_bytes();
			let header = abi::fuse_dirent {
				ino: entry.inode,
				off: (i + 1) as i64,
				namelen: name.len() as u32,
				typ: entry.kind.type_(),
			};
			buf.extend_from_slice(header.as_bytes());
			buf.extend_from_slice(name);
			buf.push(0);
		}

		// Pad for 8-byte alignment. TODO: why?
		let padding = [0u8; 8];
		buf.extend_from_slice(&padding[..(8 - buf.len() % 8)]);
		Self::Data(buf)
	}

	/// Write a response to a request to `file`.
	pub async fn write(&self, unique: u64, mut file: std::fs::File) -> Result<()> {
		match self {
			Self::Data(data) => {
				let len = data.len() + std::mem::size_of::<abi::fuse_out_header>();
				let header = abi::fuse_out_header {
					unique,
					len: len as u32,
					error: 0,
				};
				let iov = [
					IoSlice::new(header.as_bytes()),
					IoSlice::new(data.as_bytes()),
				];
				file.write_vectored(&iov)?;
			},
			Self::Error(error) => {
				let header = abi::fuse_out_header {
					unique,
					len: std::mem::size_of::<abi::fuse_out_header>() as u32,
					error: -error, // Errors are ERRNO * -1.
				};
				let iov = [IoSlice::new(header.as_bytes())];
				file.write_vectored(&iov)?;
			},
		}
		Ok(())
	}
}

/// Represents an entry in a directory that is exposed to the kernel on FUSE_READDIR requests.
pub struct DirectoryEntry<'a> {
	pub inode: u64,
	pub name: &'a str,
	pub kind: FileKind,
}

/// Represents the files we expose through FUSE.
pub enum FileKind {
	Directory,
	File { is_executable: bool },
	Symlink,
}

impl FileKind {
	/// Get the type flags.
	pub fn type_(&self) -> u32 {
		match self {
			Self::Directory => libc::S_IFDIR,
			Self::File { is_executable: _ } => libc::S_IFREG,
			Self::Symlink => libc::S_IFLNK,
		}
	}

	/// Get the file permissions. Since the filesystem is read-only this is stateless.
	pub fn permissions(&self) -> u32 {
		match self {
			Self::Directory => libc::S_IREAD | libc::S_IEXEC,
			Self::File { is_executable } if *is_executable => libc::S_IREAD | libc::S_IEXEC,
			_ => libc::S_IREAD,
		}
	}

	/// Retrieve the st_mode flags.
	pub fn mode(&self) -> u32 {
		self.type_() | self.permissions()
	}
}
