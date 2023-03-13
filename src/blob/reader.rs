use std::pin::Pin;
use tokio::io::AsyncRead;

pub struct Reader {
	pub file: tokio::fs::File,
	pub permit: tokio::sync::OwnedSemaphorePermit,
}

impl AsyncRead for Reader {
	fn poll_read(
		mut self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &mut tokio::io::ReadBuf<'_>,
	) -> std::task::Poll<std::io::Result<()>> {
		Pin::new(&mut self.file).poll_read(cx, buf)
	}
}
