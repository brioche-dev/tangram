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
