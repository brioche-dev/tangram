use std::io::{IoSlice, Write};
use zerocopy::AsBytes;

use super::abi;
use crate::{error::Result, return_error};

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
		// TODO: make async.
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
				let size = file.write_vectored(&iov)?;
				if size != header.as_bytes().len() + data.as_bytes().len() {
					return_error!("Failed to complete FUSE write.");
				}
			},
			Self::Error(error) => {
				let header = abi::fuse_out_header {
					unique,
					len: std::mem::size_of::<abi::fuse_out_header>() as u32,
					error: -error, // Errors are ERRNO * -1.
				};
				let iov = [IoSlice::new(header.as_bytes())];
				let size = file.write_vectored(&iov)?;
				if size != header.as_bytes().len() {
					return_error!("Failed to complete FUSE write.");
				}
			},
		}
		Ok(())
	}
}
