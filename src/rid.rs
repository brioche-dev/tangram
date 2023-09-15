use crate::error::{error, Error};
use byteorder::{NativeEndian, ReadBytesExt};

pub const SIZE: usize = 16;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Rid([u8; SIZE]);

impl Rid {
	#[must_use]
	pub fn gen() -> Rid {
		Rid(rand::random())
	}

	pub fn with_bytes(bytes: [u8; SIZE]) -> Self {
		Self(bytes)
	}

	#[must_use]
	pub fn as_bytes(&self) -> &[u8] {
		&self.0
	}
}

impl From<Rid> for [u8; SIZE] {
	fn from(id: Rid) -> [u8; SIZE] {
		id.0
	}
}

impl From<[u8; SIZE]> for Rid {
	fn from(id: [u8; SIZE]) -> Rid {
		Rid(id)
	}
}

impl From<Rid> for Vec<u8> {
	fn from(id: Rid) -> Vec<u8> {
		id.0.to_vec()
	}
}

impl TryFrom<Vec<u8>> for Rid {
	type Error = Error;

	fn try_from(id: Vec<u8>) -> Result<Rid, Self::Error> {
		let id = id.try_into().map_err(|_| error!("Invalid ID."))?;
		let id = Rid(id);
		Ok(id)
	}
}

impl std::fmt::Display for Rid {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", hex::encode(self.0))?;
		Ok(())
	}
}

impl std::str::FromStr for Rid {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let id = hex::decode(s)
			.map_err(|_| error!("Invalid ID."))?
			.try_into()
			.map_err(|_| error!("Invalid ID."))?;
		let id = Rid(id);
		Ok(id)
	}
}

impl serde::Serialize for Rid {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.to_string())
	}
}

impl<'de> serde::Deserialize<'de> for Rid {
	fn deserialize<D>(deserializer: D) -> Result<Rid, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		struct IdVisitor;
		impl<'de> serde::de::Visitor<'de> for IdVisitor {
			type Value = Rid;
			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				formatter.write_str("a string")
			}
			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				value.parse().map_err(|_| E::custom("Invalid ID."))
			}
		}
		deserializer.deserialize_str(IdVisitor)
	}
}

impl std::hash::Hash for Rid {
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse() {
		let s = "00000000000000000000000000000000";
		assert_eq!(s.parse::<Rid>().unwrap().to_string(), s);

		let s = "0000000000000000000000000000000z";
		s.parse::<Rid>().unwrap_err();

		let s = "f51a3a61ee9d4731b1b06c816a8ab856";
		assert_eq!(s.parse::<Rid>().unwrap().to_string(), s);

		let s = "abc123";
		s.parse::<Rid>().unwrap_err();
	}
}
