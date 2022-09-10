use crate::{artifact::Artifact, blob, hash};
use camino::Utf8PathBuf;
use derive_more::{Display, FromStr};
use std::collections::BTreeMap;

/// The hash of an [`Object`].
#[derive(
	Clone, Copy, Debug, Display, Eq, FromStr, Hash, PartialEq, serde::Deserialize, serde::Serialize,
)]
pub struct Hash(pub hash::Hash);

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_tangram")]
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
	#[must_use]
	pub fn hash(&self) -> Hash {
		Hash(hash::Hash::new(serde_json::to_vec(self).unwrap()))
	}
}

/// An object representing a directory.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Directory {
	pub entries: BTreeMap<String, Hash>,
}

/// An object representing a file.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct File {
	pub blob_hash: blob::Hash,
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
}
