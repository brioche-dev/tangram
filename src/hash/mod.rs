pub use self::{hasher::Hasher, writer::Writer};

pub mod hasher;
pub mod writer;

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
		let mut writer = Writer::new();
		writer.update(bytes.as_ref());
		writer.finalize()
	}

	#[must_use]
	pub fn as_slice(&self) -> &[u8] {
		&self.0
	}
}

impl buffalo::Serialize for Hash {
	fn serialize<W>(&self, serializer: &mut buffalo::Serializer<W>) -> std::io::Result<()>
	where
		W: std::io::Write,
	{
		serializer.serialize_bytes(self.0.as_slice())
	}
}

impl buffalo::Deserialize for Hash {
	fn deserialize<R>(deserializer: &mut buffalo::Deserializer<R>) -> std::io::Result<Self>
	where
		R: std::io::Read,
	{
		let value = deserializer.deserialize()?;
		let hash = Hash(value);
		Ok(hash)
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
		write!(f, "{hash}")?;
		Ok(())
	}
}

impl std::str::FromStr for Hash {
	type Err = hex::FromHexError;

	fn from_str(s: &str) -> Result<Hash, hex::FromHexError> {
		use hex::FromHex;
		let bytes = <[u8; 32]>::from_hex(s)?;
		Ok(Hash(bytes))
	}
}

impl TryFrom<String> for Hash {
	type Error = hex::FromHexError;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}

impl From<Hash> for String {
	fn from(value: Hash) -> Self {
		value.to_string()
	}
}

impl rand::distributions::Distribution<Hash> for rand::distributions::Standard {
	fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Hash {
		Hash(rng.gen())
	}
}

pub type BuildHasher = std::hash::BuildHasherDefault<Hasher>;
