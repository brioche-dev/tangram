use std::io::{IoSlice, Write};
use zerocopy::AsBytes;

use super::abi;
use crate::error::Result;

#[derive(Debug)]
pub enum Response {
	Error(i32),
	Data(Vec<u8>),
}

impl Response {
	pub fn error(ec: i32) -> Self {
		Self::Error(ec)
	}

	pub fn data(data: &[u8]) -> Self {
		Self::Data(data.iter().copied().collect())
	}

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

		// Pad for 8-byte alignment.
		let padding = [0u8; 8];
		buf.extend_from_slice(&padding[..(8 - buf.len() % 8)]);
		Self::Data(buf)
	}

	#[tracing::instrument]
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
					error: *error,
				};
				let iov = [IoSlice::new(header.as_bytes())];
				file.write_vectored(&iov)?;
			},
		}
		Ok(())
	}
}

pub struct DirectoryEntry<'a> {
	pub inode: u64,
	pub name: &'a str,
	pub kind: FileKind,
}

pub enum FileKind {
	Directory,
	File { is_executable: bool },
	Symlink,
}

impl FileKind {
	pub fn type_(&self) -> u32 {
		match self {
			Self::Directory => libc::S_IFDIR,
			Self::File { is_executable: _ } => libc::S_IFREG,
			Self::Symlink => libc::S_IFLNK,
		}
	}

	pub fn permissions(&self) -> u32 {
		match self {
			Self::File { is_executable } if *is_executable => libc::S_IREAD | libc::S_IEXEC,
			_ => libc::S_IREAD,
		}
	}

	pub fn mode(&self) -> u32 {
		self.type_() | self.permissions()
	}
}
