use super::{Algorithm, Checksum};

#[derive(Debug)]
pub enum Writer {
	Blake3(Box<blake3::Hasher>),
	Sha256(sha2::Sha256),
	Sha512(sha2::Sha512),
}

impl Writer {
	#[must_use]
	pub fn new(algorithm: Algorithm) -> Writer {
		match algorithm {
			Algorithm::Blake3 => Writer::Blake3(Box::new(blake3::Hasher::new())),
			Algorithm::Sha256 => Writer::Sha256(sha2::Sha256::default()),
			Algorithm::Sha512 => Writer::Sha512(sha2::Sha512::default()),
		}
	}

	pub fn update(&mut self, data: impl AsRef<[u8]>) {
		match self {
			Writer::Blake3(hasher) => {
				hasher.update(data.as_ref());
			},
			Writer::Sha256(sha256) => {
				sha2::Digest::update(sha256, data);
			},
			Writer::Sha512(sha512) => {
				sha2::Digest::update(sha512, data);
			},
		}
	}

	#[must_use]
	pub fn finalize(self) -> Checksum {
		match self {
			Writer::Blake3(hasher) => {
				let value = hasher.finalize();
				Checksum::Blake3(value.into())
			},
			Writer::Sha256(sha256) => {
				let value = sha2::Digest::finalize(sha256);
				Checksum::Sha256(value.into())
			},
			Writer::Sha512(sha512) => {
				let value = sha2::Digest::finalize(sha512);
				Checksum::Sha512(value.into())
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
