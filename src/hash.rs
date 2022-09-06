use anyhow::Context;
use digest::Digest;
use tokio::io::AsyncWrite;

#[derive(Default)]
pub struct Hasher {
	sha256: sha2::Sha256,
}

impl Hasher {
	#[must_use]
	pub fn new() -> Hasher {
		Hasher::default()
	}

	pub fn update(&mut self, data: impl AsRef<[u8]>) {
		self.sha256.update(data);
	}

	#[must_use]
	pub fn finalize(self) -> Hash {
		Hash(self.sha256.finalize().into())
	}
}

impl std::io::Write for Hasher {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		self.update(buf);
		Ok(buf.len())
	}

	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}

impl AsyncWrite for Hasher {
	fn poll_write(
		mut self: std::pin::Pin<&mut Self>,
		_cx: &mut std::task::Context<'_>,
		buf: &[u8],
	) -> std::task::Poll<Result<usize, std::io::Error>> {
		self.sha256.update(&buf);
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

#[derive(
	Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Deserialize, serde::Serialize,
)]
pub struct Hash(#[serde(with = "hex")] pub [u8; 32]);

impl Hash {
	#[must_use]
	pub fn zero() -> Hash {
		Hash([0; 32])
	}

	pub fn new(bytes: impl AsRef<[u8]>) -> Hash {
		let mut hasher = Hasher::new();
		hasher.update(bytes.as_ref());
		hasher.finalize()
	}

	/// Get the hash data as a byte slice.
	#[must_use]
	pub fn as_slice(&self) -> &[u8] {
		&self.0
	}
}

impl TryFrom<&[u8]> for Hash {
	type Error = anyhow::Error;

	fn try_from(slice: &[u8]) -> anyhow::Result<Hash> {
		let data: [u8; 32] = slice.try_into().with_context(|| {
			format!(
				"could not create Hash from {}-byte slice (expected 32)",
				slice.len()
			)
		})?;
		Ok(Hash(data))
	}
}

impl From<digest::Output<sha2::Sha256>> for Hash {
	fn from(value: digest::Output<sha2::Sha256>) -> Self {
		Hash(value.into())
	}
}

impl std::fmt::Debug for Hash {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let hash = hex::encode(self.0);
		f.debug_tuple("Hash").field(&hash).finish()
	}
}

impl std::fmt::Display for Hash {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let hash = hex::encode(self.0);
		write!(f, "{hash}")
	}
}

impl std::str::FromStr for Hash {
	type Err = hex::FromHexError;
	fn from_str(source: &str) -> Result<Hash, hex::FromHexError> {
		use hex::FromHex;
		let bytes = <[u8; 32]>::from_hex(source)?;
		Ok(Hash(bytes))
	}
}

impl rand::distributions::Distribution<Hash> for rand::distributions::Standard {
	fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Hash {
		Hash(rng.gen())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn display_fromstr_roundtrip() {
		let message = "Hello, World!";
		let mut hasher = Hasher::new();
		hasher.update(&message);
		let left = hasher.finalize();
		let right = left.to_string().parse().unwrap();
		assert_eq!(left, right);
	}
}
