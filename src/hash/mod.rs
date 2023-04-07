pub use self::{hasher::Hasher, writer::Writer};
use derive_more::{From, Into};

pub mod hasher;
pub mod writer;

#[derive(
	Clone,
	Copy,
	Default,
	Eq,
	From,
	Hash,
	Into,
	Ord,
	PartialEq,
	PartialOrd,
	buffalo::Serialize,
	buffalo::Deserialize,
	serde::Serialize,
	serde::Deserialize,
)]
#[buffalo(into = "[u8; 32]", try_from = "[u8; 32]")]
pub struct Hash(#[serde(with = "hex")] pub [u8; 32]);

impl Hash {
	#[must_use]
	pub fn zero() -> Self {
		Self([0; 32])
	}

	pub fn new(bytes: impl AsRef<[u8]>) -> Self {
		let mut writer = Writer::new();
		writer.update(bytes.as_ref());
		writer.finalize()
	}

	#[must_use]
	pub fn as_slice(&self) -> &[u8] {
		&self.0
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
