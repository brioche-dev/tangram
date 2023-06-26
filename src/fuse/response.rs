use bytes::BufMut;
use tokio::io::AsyncWriteExt;
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
	pub async fn write(&self, unique: u64, file: &mut tokio::fs::File) -> Result<()> {
		let len = match &self {
			Response::Error(_) => 0,
			Response::Data(data) => data.len(),
		};

		let header = abi::fuse_out_header {
			unique,
			error: if let Response::Error(ec) = self {
				*ec
			} else {
				0
			},
			len: (std::mem::size_of::<abi::fuse_out_header>() + len)
				.try_into()
				.unwrap(),
		};

		let header = header.as_bytes();
		let mut response = Vec::with_capacity(header.len() + len);
		response.put_slice(header);

		// TODO: use write_vectored here.
		if let Self::Data(data) = &self {
			response.put_slice(data);
		}

		file.write_all(&response).await?;
		file.flush().await?;
		eprintln!("Done writing response.");
		Ok(())
	}
}
