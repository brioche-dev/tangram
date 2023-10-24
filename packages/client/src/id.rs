use crate::{return_error, Error, Result, WrapErr};
use bytes::Bytes;
use derive_more::{From, Into};
use varint_rs::{VarintReader, VarintWriter};

/// An ID.
#[derive(
	Clone,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(into = "String", try_from = "String")]
#[tangram_serialize(into = "String", try_from = "String")]
pub enum Id {
	V0(V0),
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct V0 {
	kind: Kind,
	hash: Hash,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Kind {
	Blob,
	Directory,
	File,
	Symlink,
	Package,
	Target,
	Build,
	User,
	Login,
	Token,
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Hash {
	Random32([u8; 32]),
	Blake3([u8; 32]),
}

impl Id {
	#[must_use]
	pub fn new_random(kind: Kind) -> Self {
		let hash = Hash::Random32(rand::random());
		Self::V0(V0 { kind, hash })
	}

	#[must_use]
	pub fn new_hashed(kind: Kind, bytes: &[u8]) -> Self {
		let hash = blake3::hash(bytes);
		let hash = Hash::Blake3(*hash.as_bytes());
		Self::V0(V0 { kind, hash })
	}

	pub fn with_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
		let mut bytes = bytes.as_ref();
		let version = bytes.read_u64_varint().wrap_err("Invalid version.")?;
		if version != 0 {
			return_error!("This version of the client does not support this ID version.");
		}
		let kind = bytes.read_u64_varint().wrap_err("Invalid kind.")?;
		let kind = Kind::try_from(kind).wrap_err("Invalid kind.")?;
		let hash = Hash::from_reader(&mut bytes).wrap_err("Invalid hash.")?;
		Ok(Self::V0(V0 { kind, hash }))
	}

	#[must_use]
	pub fn to_bytes(&self) -> Bytes {
		match self {
			Id::V0(v0) => {
				let mut bytes = Vec::new();
				bytes.write_u64_varint(0).unwrap();
				bytes.write_u64_varint(u64::from(v0.kind)).unwrap();
				v0.hash.to_writer(&mut bytes).unwrap();
				bytes.into()
			},
		}
	}

	#[must_use]
	pub fn kind(&self) -> Kind {
		match self {
			Id::V0(v0) => v0.kind,
		}
	}
}

impl Hash {
	pub fn from_reader(reader: &mut impl std::io::Read) -> std::io::Result<Self> {
		let kind = reader.read_u64_varint()?;
		match kind {
			0 => {
				let mut bytes = [0u8; 32];
				reader.read_exact(&mut bytes)?;
				Ok(Self::Random32(bytes))
			},
			1 => {
				let mut bytes = [0u8; 32];
				reader.read_exact(&mut bytes)?;
				Ok(Self::Blake3(bytes))
			},
			_ => Err(std::io::Error::new(
				std::io::ErrorKind::Other,
				"Invalid hash kind.",
			)),
		}
	}

	pub fn to_writer(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
		match self {
			Self::Random32(bytes) => {
				writer.write_u64_varint(0)?;
				writer.write_all(bytes)?;
			},
			Self::Blake3(bytes) => {
				writer.write_u64_varint(1)?;
				writer.write_all(bytes)?;
			},
		}
		Ok(())
	}
}

impl std::fmt::Debug for Id {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("Id").field(&self.to_string()).finish()
	}
}

impl std::fmt::Display for Id {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let kind = match self.kind() {
			Kind::Blob => "blb",
			Kind::Directory => "dir",
			Kind::File => "fil",
			Kind::Symlink => "sym",
			Kind::Package => "pkg",
			Kind::Target => "tgt",
			Kind::Build => "bld",
			Kind::User => "usr",
			Kind::Login => "lgn",
			Kind::Token => "tok",
		};
		write!(f, "{kind}_")?;
		let bytes = self.to_bytes();
		write!(f, "{}", hex::encode(bytes))?;
		Ok(())
	}
}

impl std::str::FromStr for Id {
	type Err = Error;

	fn from_str(string: &str) -> Result<Self, Self::Err> {
		let string = string.get(4..).wrap_err("Invalid ID.")?;
		let bytes = hex::decode(string).wrap_err("Invalid ID.")?;
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

impl TryFrom<Vec<u8>> for Id {
	type Error = Error;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		value.as_slice().try_into()
	}
}

impl From<Id> for Vec<u8> {
	fn from(value: Id) -> Self {
		value.to_bytes().to_vec()
	}
}

impl TryFrom<&[u8]> for Id {
	type Error = Error;

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		Self::with_bytes(value)
	}
}

impl From<Kind> for u64 {
	fn from(value: Kind) -> Self {
		match value {
			Kind::Blob => 0,
			Kind::Directory => 1,
			Kind::File => 2,
			Kind::Symlink => 3,
			Kind::Package => 4,
			Kind::Target => 5,
			Kind::Build => 6,
			Kind::User => 7,
			Kind::Login => 8,
			Kind::Token => 9,
		}
	}
}

impl TryFrom<u64> for Kind {
	type Error = Error;

	fn try_from(value: u64) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(Kind::Blob),
			1 => Ok(Kind::Directory),
			2 => Ok(Kind::File),
			3 => Ok(Kind::Symlink),
			4 => Ok(Kind::Package),
			5 => Ok(Kind::Target),
			6 => Ok(Kind::Build),
			7 => Ok(Kind::User),
			8 => Ok(Kind::Login),
			9 => Ok(Kind::Token),
			_ => return_error!("Invalid kind."),
		}
	}
}
