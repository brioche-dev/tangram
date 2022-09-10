use crate::hash;
use derive_more::{Display, FromStr};

/// The hash of a Blob.
#[derive(
	Clone, Copy, Debug, Display, Eq, FromStr, Hash, PartialEq, serde::Deserialize, serde::Serialize,
)]
pub struct Hash(pub hash::Hash);

/// A Blob is the contents of a file.
#[derive(Clone, Debug)]
pub struct Blob(pub Vec<u8>);

impl Blob {
	#[must_use]
	pub fn hash(&self) -> Hash {
		Hash(hash::Hash::new(serde_json::to_vec(self).unwrap()))
	}
}

impl serde::ser::Serialize for Blob {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::ser::Serializer,
	{
		let string = base64::encode(&self.0);
		serializer.serialize_str(&string)
	}
}

impl<'de> serde::de::Deserialize<'de> for Blob {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::de::Deserializer<'de>,
	{
		struct Visitor;
		impl<'de> serde::de::Visitor<'de> for Visitor {
			type Value = Blob;
			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				formatter.write_str("a string")
			}
			fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				let value = base64::decode(value)
					.map_err(|error| serde::de::Error::custom(error.to_string()))?;
				Ok(Blob(value))
			}
		}
		deserializer.deserialize_str(Visitor)
	}
}
