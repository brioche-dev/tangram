use crate::hash::Hash;

#[allow(clippy::module_name_repetitions)]
#[derive(
	Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Deserialize, serde::Serialize,
)]
pub struct PackageHash(pub Hash);

impl std::ops::Deref for PackageHash {
	type Target = Hash;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl std::fmt::Debug for PackageHash {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl std::fmt::Display for PackageHash {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl std::str::FromStr for PackageHash {
	type Err = hex::FromHexError;
	fn from_str(source: &str) -> Result<Self, hex::FromHexError> {
		Ok(Self(Hash::from_str(source)?))
	}
}

impl buffalo::Serialize for PackageHash {
	fn serialize<W>(&self, serializer: &mut buffalo::Serializer<W>) -> std::io::Result<()>
	where
		W: std::io::Write,
	{
		serializer.serialize(&self.0)
	}
}

impl buffalo::Deserialize for PackageHash {
	fn deserialize<R>(deserializer: &mut buffalo::Deserializer<R>) -> std::io::Result<Self>
	where
		R: std::io::Read,
	{
		Ok(Self(deserializer.deserialize()?))
	}
}
