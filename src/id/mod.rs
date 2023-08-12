pub use self::hasher::Hasher;
use derive_more::{From, Into};

pub mod hasher;

#[derive(
	Clone,
	Copy,
	Eq,
	From,
	Into,
	Ord,
	PartialEq,
	PartialOrd,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(into = "String", try_from = "String")]
#[tangram_serialize(into = "[u8; 32]", try_from = "[u8; 32]")]
pub struct Id([u8; 32]);

impl Id {
	#[must_use]
	pub fn with_bytes(bytes: &[u8]) -> Id {
		let hash = blake3::hash(bytes);
		let hash = *hash.as_bytes();
		Id(hash)
	}

	#[must_use]
	pub fn as_bytes(&self) -> [u8; 32] {
		self.0
	}
}

impl std::fmt::Debug for Id {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let hex = hex::encode(self.0);
		f.debug_tuple("Id").field(&hex).finish()
	}
}

impl std::fmt::Display for Id {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let hex = hex::encode(self.0);
		write!(f, "{hex}")?;
		Ok(())
	}
}

impl std::str::FromStr for Id {
	type Err = hex::FromHexError;

	fn from_str(s: &str) -> Result<Id, hex::FromHexError> {
		use hex::FromHex;
		let bytes = <_>::from_hex(s)?;
		Ok(Id(bytes))
	}
}

impl From<Id> for String {
	fn from(value: Id) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Id {
	type Error = hex::FromHexError;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}

impl rand::distributions::Distribution<Id> for rand::distributions::Standard {
	fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Id {
		Id(rng.gen())
	}
}

impl std::hash::Hash for Id {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		state.write(&self.0);
	}
}

pub type BuildHasher = std::hash::BuildHasherDefault<Hasher>;
