use tokio::io::AsyncWriteExt;
use zerocopy::AsBytes;

use super::abi;
use crate::error::Result;

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

		// TODO: use write_vectored here.
		file.write_all(header.as_bytes()).await?;
		if let Self::Data(data) = &self {
			file.write_all(data).await?;
		}

		Ok(())
	}
}
