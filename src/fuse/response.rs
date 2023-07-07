use std::io::Write;
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
		Self::Data(data.to_vec())
	}

	/// Write a response to a request to `file`.
	pub async fn write(&self, unique: u64, mut file: std::fs::File) -> Result<()> {
		// TODO: make async. We cannot use tokio::fs::File::write, write_all, write_vectored because:
		// - write_vectored only writes the first non-empty IoSlice.
		// - tokio::fs::file::write assumes that the underlying file descriptor is a file.
		let buffer = match self {
			Self::Data(data) => {
				let len = data.len() + std::mem::size_of::<abi::fuse_out_header>();
				let header = abi::fuse_out_header {
					unique,
					len: len as u32,
					error: 0,
				};

				let mut buffer = header.as_bytes().to_owned();
				buffer.extend_from_slice(data);
				buffer
			},
			Self::Error(error) => {
				let header = abi::fuse_out_header {
					unique,
					len: std::mem::size_of::<abi::fuse_out_header>() as u32,
					error: -error, // Errors are ERRNO * -1.
				};
				header.as_bytes().to_owned()
			},
		};

		match file.write_all(&buffer) {
			Ok(_) => (),
			// ENOENT means the kernel will retry the request.
			Err(e) if e.raw_os_error() == Some(libc::ENOENT) => (),
			Err(e) => {
				let buffer_len = buffer.len();
				tracing::error!(?e, ?buffer_len, "Failed to write FUSE result.");
				Err(e)?;
			},
		}
		Ok(())
	}
}
