use crate::{artifact::Artifact, hash::Hash};
use camino::Utf8PathBuf;
use derive_more::{Deref, Display, FromStr};
use std::collections::BTreeMap;

/// The hash of an [`Object`].
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
pub struct ObjectHash(Hash);

/// A filesystem object, either a directory, file, or symlink.
pub enum Object {
	/// A directory.
	Directory(Directory),

	/// A file.
	File(File),

	/// A symbolic link.
	Symlink(Symlink),

	/// A dependency.
	Dependency(Dependency),
}

/// An object representing a directory.
pub struct Directory {
	pub entries: BTreeMap<String, ObjectHash>,
}

/// An object representing a file.
pub struct File {
	pub blob_hash: BlobHash,
	pub executable: bool,
}

/// An object representing a symbolic link.
pub struct Symlink {
	pub content: Utf8PathBuf,
}

/// An object representing a symbolic link.
pub struct Dependency {
	pub artifact: Artifact,
	pub path: Utf8PathBuf,
}

/// The hash of a Blob.
pub struct BlobHash(Hash);

/// A Blob is the contents of a file.
pub struct Blob(Vec<u8>);
