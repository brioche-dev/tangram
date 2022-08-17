use crate::{artifact::Artifact, hash::Hash};
use camino::Utf8PathBuf;
use derive_more::{Deref, Display, FromStr};
use std::collections::BTreeMap;

/// The hash of an [`Object`].
#[allow(clippy::module_name_repetitions)]
#[derive(
	Clone,
	Copy,
	Debug,
	Deref,
	Display,
	Eq,
	FromStr,
	Hash,
	PartialEq,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct ObjectHash(pub Hash);

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Object {
	/// A directory.
	#[serde(rename = "directory")]
	Directory(Directory),

	/// A file.
	#[serde(rename = "file")]
	File(File),

	/// A symbolic link.
	#[serde(rename = "symlink")]
	Symlink(Symlink),

	/// A dependency.
	#[serde(rename = "dependency")]
	Dependency(Dependency),
}

impl Object {
	pub(crate) fn hash(&self) -> ObjectHash {
		ObjectHash(Hash::new(serde_json::to_vec(self).unwrap()))
	}
}

/// An object representing a directory.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Directory {
	pub entries: BTreeMap<String, ObjectHash>,
}

/// An object representing a file.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct File {
	pub blob_hash: BlobHash,
	pub executable: bool,
}

/// An object representing a symbolic link.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Symlink {
	pub target: Utf8PathBuf,
}

/// An object representing a dependency on another artifact.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Dependency {
	pub artifact: Artifact,
	pub path: Utf8PathBuf,
}

/// The hash of a Blob.
#[derive(
	Clone,
	Copy,
	Debug,
	Deref,
	Display,
	Eq,
	FromStr,
	Hash,
	PartialEq,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct BlobHash(pub Hash);

/// A Blob is the contents of a file.
#[derive(Clone, Debug)]
pub struct Blob(pub Vec<u8>);

impl Blob {
	#[must_use]
	pub fn hash(&self) -> BlobHash {
		BlobHash(Hash::new(serde_json::to_vec(self).unwrap()))
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
