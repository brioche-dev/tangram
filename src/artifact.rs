use crate::hash::Hash;
use anyhow::{bail, Result};
use byteorder::{ReadBytesExt, WriteBytesExt};
use camino::Utf8PathBuf;
use std::collections::BTreeMap;

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(tag = "type", content = "value")]
pub enum Artifact {
	#[buffalo(id = 0)]
	#[serde(rename = "directory")]
	Directory(Directory),

	#[buffalo(id = 1)]
	#[serde(rename = "file")]
	File(File),

	#[buffalo(id = 2)]
	#[serde(rename = "symlink")]
	Symlink(Symlink),

	#[buffalo(id = 3)]
	#[serde(rename = "dependency")]
	Dependency(Dependency),
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Directory {
	#[buffalo(id = 0)]
	pub entries: BTreeMap<String, Hash>,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct File {
	#[buffalo(id = 0)]
	pub blob: Hash,

	#[buffalo(id = 1)]
	pub executable: bool,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Symlink {
	#[buffalo(id = 0)]
	pub target: Utf8PathBuf,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Dependency {
	#[buffalo(id = 0)]
	pub artifact: Hash,

	#[buffalo(id = 1)]
	pub path: Option<Utf8PathBuf>,
}

impl Artifact {
	pub fn deserialize<R>(mut reader: R) -> Result<Artifact>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			bail!(r#"Cannot deserialize expression with version "{version}"."#);
		}

		// Deserialize the expression.
		let expression = buffalo::from_reader(reader)?;

		Ok(expression)
	}

	pub fn deserialize_from_slice(slice: &[u8]) -> Result<Artifact> {
		Artifact::deserialize(slice)
	}

	pub fn serialize<W>(&self, mut writer: W) -> Result<()>
	where
		W: std::io::Write,
	{
		// Write the version.
		writer.write_u8(0)?;

		// Write the expression.
		buffalo::to_writer(self, &mut writer)?;

		Ok(())
	}

	#[must_use]
	pub fn serialize_to_vec(&self) -> Vec<u8> {
		let mut data = Vec::new();
		self.serialize(&mut data).unwrap();
		data
	}

	#[must_use]
	pub fn serialize_to_vec_and_hash(&self) -> (Hash, Vec<u8>) {
		let data = self.serialize_to_vec();
		let hash = Hash::new(&data);
		(hash, data)
	}

	#[must_use]
	pub fn hash(&self) -> Hash {
		let data = self.serialize_to_vec();
		Hash::new(&data)
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

	#[must_use]
	pub fn as_dependency(&self) -> Option<&Dependency> {
		if let Artifact::Dependency(v) = self {
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

	#[must_use]
	pub fn into_dependency(self) -> Option<Dependency> {
		if let Artifact::Dependency(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AddArtifactOutcome {
	Added { hash: Hash },
	DirectoryMissingEntries { entries: Vec<(String, Hash)> },
	FileMissingBlob { blob_hash: Hash },
	DependencyMissing { hash: Hash },
}
