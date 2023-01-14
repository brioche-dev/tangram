pub use self::hash::BlobHash;
use std::pin::Pin;
use tokio::io::AsyncRead;

mod add;
mod copy;
mod get;
mod hash;

pub struct Blob {
	_permit: tokio::sync::OwnedSemaphorePermit,
	file: tokio::fs::File,
}

impl Blob {
	pub fn new(permit: tokio::sync::OwnedSemaphorePermit, file: tokio::fs::File) -> Blob {
		Blob {
			_permit: permit,
			file,
		}
	}
}

impl AsyncRead for Blob {
	fn poll_read(
		mut self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &mut tokio::io::ReadBuf<'_>,
	) -> std::task::Poll<std::io::Result<()>> {
		Pin::new(&mut self.file).poll_read(cx, buf)
	}
}
