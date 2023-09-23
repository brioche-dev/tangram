use crate::{return_error, Error, Result, WrapErr};
use byteorder::{NativeEndian, ReadBytesExt};
use derive_more::{From, Into};

pub const SIZE: usize = 32;

#[derive(Clone, Copy, Debug)]
pub enum Kind {
	Blob,
	Directory,
	File,
	Symlink,
	Package,
	Task,
	Run,
}

/// An ID.
#[derive(
	Clone,
	Copy,
	Eq,
	Ord,
	From,
	Into,
	PartialEq,
	PartialOrd,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(into = "String", try_from = "String")]
#[tangram_serialize(into = "[u8; SIZE]", try_from = "[u8; SIZE]")]
pub struct Id([u8; SIZE]);

impl Id {
	#[must_use]
	pub fn new_random(kind: Kind) -> Self {
		let mut bytes: [u8; SIZE] = rand::random();
		bytes[0] = kind.into();
		Self(bytes)
	}

	#[must_use]
	pub fn new_hashed(kind: Kind, data: &[u8]) -> Self {
		let hash = blake3::hash(data);
		let mut bytes = *hash.as_bytes();
		bytes[0] = kind.into();
		Self(bytes)
	}

	pub fn with_bytes(bytes: [u8; SIZE]) -> Result<Self> {
		Kind::try_from(bytes[0]).wrap_err("Invalid kind.")?;
		Ok(Self(bytes))
	}

	#[must_use]
	pub fn as_bytes(&self) -> [u8; SIZE] {
		self.0
	}

	#[must_use]
	pub fn kind(&self) -> Kind {
		self.0[0].try_into().unwrap()
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
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		use hex::FromHex;
		let bytes = <_>::from_hex(s).map_err(Error::other)?;
		let id = Self::with_bytes(bytes)?;
		Ok(id)
	}
}

impl From<Id> for String {
	fn from(value: Id) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Id {
	type Error = Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}

impl std::hash::Hash for Id {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		state.write(&self.0);
	}
}

#[derive(Default)]
pub struct Hasher(Option<u64>);

impl std::hash::Hasher for Hasher {
	fn finish(&self) -> u64 {
		self.0.unwrap()
	}

	fn write(&mut self, mut bytes: &[u8]) {
		assert!(self.0.is_none());
		assert_eq!(bytes.len(), SIZE);
		let value = bytes.read_u64::<NativeEndian>().unwrap();
		self.0 = Some(value);
	}
}

pub type BuildHasher = std::hash::BuildHasherDefault<Hasher>;

impl From<Kind> for u8 {
	fn from(value: Kind) -> Self {
		match value {
			Kind::Blob => 0,
			Kind::Directory => 1,
			Kind::File => 2,
			Kind::Symlink => 3,
			Kind::Package => 4,
			Kind::Task => 5,
			Kind::Run => 6,
		}
	}
}

impl TryFrom<u8> for Kind {
	type Error = Error;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(Kind::Blob),
			1 => Ok(Kind::Directory),
			2 => Ok(Kind::File),
			3 => Ok(Kind::Symlink),
			4 => Ok(Kind::Package),
			5 => Ok(Kind::Task),
			6 => Ok(Kind::Run),
			_ => return_error!("Invalid kind."),
		}
	}
}
