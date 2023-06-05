pub use self::{data::Data, hash::Hash, tracker::Tracker};
use crate::{directory::Directory, file::File, symlink::Symlink};

mod bundle;
pub mod checkin;
mod checkout;
mod checksum;
mod data;
mod get;
mod hash;
mod references;
mod tracker;

/// An artifact.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Artifact {
	/// A directory.
	Directory(Directory),

	/// A file.
	File(File),

	/// A symlink.
	Symlink(Symlink),
}

impl From<Directory> for Artifact {
	fn from(directory: Directory) -> Self {
		Self::Directory(directory)
	}
}

impl From<File> for Artifact {
	fn from(file: File) -> Self {
		Self::File(file)
	}
}

impl From<Symlink> for Artifact {
	fn from(symlink: Symlink) -> Self {
		Self::Symlink(symlink)
	}
}

impl Artifact {
	#[must_use]
	pub fn as_directory(&self) -> Option<&Directory> {
		if let Artifact::Directory(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_file(&self) -> Option<&File> {
		if let Artifact::File(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_symlink(&self) -> Option<&Symlink> {
		if let Artifact::Symlink(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

impl Artifact {
	#[must_use]
	pub fn into_directory(self) -> Option<Directory> {
		if let Artifact::Directory(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_file(self) -> Option<File> {
		if let Artifact::File(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_symlink(self) -> Option<Symlink> {
		if let Artifact::Symlink(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

impl Artifact {
	#[must_use]
	pub fn hash(&self) -> Hash {
		match self {
			Self::Directory(directory) => directory.hash(),
			Self::File(file) => file.hash(),
			Self::Symlink(symlink) => symlink.hash(),
		}
	}
}

impl std::hash::Hash for Artifact {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.hash().hash(state);
	}
}

impl std::fmt::Display for Artifact {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Artifact::Directory(directory) => f.write_str(&format!(
				r#"(tg.directory {value})"#,
				value = directory.hash()
			)),
			Artifact::File(file) => {
				f.write_str(&format!(r#"(tg.file {value})"#, value = file.hash()))
			},
			Artifact::Symlink(symlink) => {
				f.write_str(&format!(r#"(tg.symlink {value})"#, value = symlink.hash()))
			},
		}
	}
}
