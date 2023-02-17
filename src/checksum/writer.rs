use super::{Algorithm, Checksum};

#[derive(Debug)]
pub enum Writer {
	Sha256(sha2::Sha256),
	Blake3(Box<blake3::Hasher>),
}

impl Writer {
	#[must_use]
	pub fn new(algorithm: Algorithm) -> Writer {
		match algorithm {
			Algorithm::Sha256 => Writer::Sha256(sha2::Sha256::default()),
			Algorithm::Blake3 => Writer::Blake3(Box::new(blake3::Hasher::new())),
		}
	}

	pub fn update(&mut self, data: impl AsRef<[u8]>) {
		match self {
			Writer::Sha256(sha256) => {
				sha2::Digest::update(sha256, data);
			},
			Writer::Blake3(hasher) => {
				hasher.update(data.as_ref());
			},
		}
	}

	#[must_use]
	pub fn finalize(self) -> Checksum {
		match self {
			Writer::Sha256(sha256) => {
				let value = sha2::Digest::finalize(sha256);
				Checksum::Sha256(value.into())
			},
			Writer::Blake3(hasher) => {
				let value = hasher.finalize();
				Checksum::Blake3(value.into())
			},
		}
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
