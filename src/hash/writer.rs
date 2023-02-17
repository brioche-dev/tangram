use super::Hash;

#[derive(Default)]
pub struct Writer {
	hasher: blake3::Hasher,
}

impl Writer {
	#[must_use]
	pub fn new() -> Writer {
		Writer::default()
	}

	pub fn update(&mut self, data: impl AsRef<[u8]>) {
		self.hasher.update(data.as_ref());
	}

	#[must_use]
	pub fn finalize(self) -> Hash {
		Hash(self.hasher.finalize().into())
	}
}

impl std::io::Write for Writer {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		self.update(buf);
		Ok(buf.len())
	}

	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}

impl tokio::io::AsyncWrite for Writer {
	fn poll_write(
		mut self: std::pin::Pin<&mut Self>,
		_cx: &mut std::task::Context<'_>,
		buf: &[u8],
	) -> std::task::Poll<Result<usize, std::io::Error>> {
		self.update(buf);
		std::task::Poll::Ready(Ok(buf.len()))
	}

	fn poll_flush(
		self: std::pin::Pin<&mut Self>,
		_cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), std::io::Error>> {
		std::task::Poll::Ready(Ok(()))
	}

	fn poll_shutdown(
		self: std::pin::Pin<&mut Self>,
		_cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), std::io::Error>> {
		std::task::Poll::Ready(Ok(()))
	}
}
