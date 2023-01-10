use crate::hash::Hash;
use derive_more::{Deref, Display, FromStr};

#[derive(
	Clone,
	Copy,
	Debug,
	Default,
	Deref,
	Display,
	Eq,
	FromStr,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct OperationHash(pub Hash);

impl buffalo::Serialize for OperationHash {
	fn serialize<W>(&self, serializer: &mut buffalo::Serializer<W>) -> std::io::Result<()>
	where
		W: std::io::Write,
	{
		serializer.serialize(&self.0)
	}
}

impl buffalo::Deserialize for OperationHash {
	fn deserialize<R>(deserializer: &mut buffalo::Deserializer<R>) -> std::io::Result<Self>
	where
		R: std::io::Read,
	{
		Ok(Self(deserializer.deserialize()?))
	}
}
