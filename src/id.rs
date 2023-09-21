use crate::{return_error, value::Kind, Error, Result, WrapErr};
use byteorder::{NativeEndian, ReadBytesExt};
use derive_more::{From, Into};

pub const SIZE: usize = 32;

/// A value ID.
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

// #[derive(Clone, Copy, Debug)]
// pub enum Kind {
// 	Null,
// 	Bool,
// 	Number,
// 	String,
// 	Bytes,
// 	Relpath,
// 	Subpath,
// 	Blob,
// 	Directory,
// 	File,
// 	Symlink,
// 	Placeholder,
// 	Template,
// 	Package,
// 	Resource,
// 	Target,
// 	Task,
// 	Array,
// 	Object,
// }

impl Id {
	#[must_use]
	pub fn new_random(kind: Kind) -> Self {
		Self(rand::random())
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
			Kind::Null => 0,
			Kind::Bool => 1,
			Kind::Number => 2,
			Kind::String => 3,
			Kind::Bytes => 4,
			Kind::Relpath => 5,
			Kind::Subpath => 6,
			Kind::Blob => 7,
			Kind::Directory => 8,
			Kind::File => 9,
			Kind::Symlink => 10,
			Kind::Placeholder => 11,
			Kind::Template => 12,
			Kind::Package => 13,
			Kind::Resource => 14,
			Kind::Target => 15,
			Kind::Task => 16,
			Kind::Array => 17,
			Kind::Object => 18,
		}
	}
}

impl TryFrom<u8> for Kind {
	type Error = Error;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(Kind::Null),
			1 => Ok(Kind::Bool),
			2 => Ok(Kind::Number),
			3 => Ok(Kind::String),
			4 => Ok(Kind::Bytes),
			5 => Ok(Kind::Relpath),
			6 => Ok(Kind::Subpath),
			7 => Ok(Kind::Blob),
			8 => Ok(Kind::Directory),
			9 => Ok(Kind::File),
			10 => Ok(Kind::Symlink),
			11 => Ok(Kind::Placeholder),
			12 => Ok(Kind::Template),
			13 => Ok(Kind::Package),
			14 => Ok(Kind::Resource),
			15 => Ok(Kind::Target),
			16 => Ok(Kind::Task),
			17 => Ok(Kind::Array),
			18 => Ok(Kind::Object),
			_ => return_error!("Invalid kind."),
		}
	}
}
#[macro_export]
macro_rules! id {
	() => {
		#[derive(
			Clone,
			Copy,
			Debug,
			Eq,
			Ord,
			PartialEq,
			PartialOrd,
			serde::Deserialize,
			serde::Serialize,
			tangram_serialize::Deserialize,
			tangram_serialize::Serialize,
		)]
		#[tangram_serialize(into = "crate::Id", try_from = "crate::Id")]
		pub struct Id($crate::Id);

		impl std::ops::Deref for Id {
			type Target = $crate::Id;

			fn deref(&self) -> &Self::Target {
				&self.0
			}
		}

		impl std::hash::Hash for Id {
			fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
				std::hash::Hash::hash(&self.0, state);
			}
		}

		impl std::fmt::Display for Id {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				write!(f, "{}", self.0)?;
				Ok(())
			}
		}

		impl std::str::FromStr for Id {
			type Err = $crate::Error;

			fn from_str(s: &str) -> Result<Self, Self::Err> {
				Ok(Self($crate::Id::from_str(s)?))
			}
		}
	};

	($t:ident) => {
		$crate::id!();

		impl From<Id> for $crate::Id {
			fn from(value: Id) -> Self {
				value.0
			}
		}

		impl TryFrom<$crate::Id> for Id {
			type Error = $crate::Error;

			fn try_from(value: $crate::Id) -> Result<Self, Self::Error> {
				match value.kind() {
					$crate::value::Kind::$t => Ok(Self(value)),
					_ => $crate::return_error!("Unexpected kind."),
				}
			}
		}
	};
}
