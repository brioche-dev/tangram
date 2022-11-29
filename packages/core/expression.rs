use crate::{checksum::Checksum, hash::Hash, system::System};
use anyhow::{bail, Result};
use byteorder::{ReadBytesExt, WriteBytesExt};
use camino::Utf8PathBuf;
use std::{collections::BTreeMap, sync::Arc};
use url::Url;

#[derive(
	Clone,
	Debug,
	PartialEq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(tag = "type", content = "value")]
pub enum Expression {
	#[buffalo(id = 0)]
	#[serde(rename = "null")]
	Null(()),

	#[buffalo(id = 1)]
	#[serde(rename = "bool")]
	Bool(bool),

	#[buffalo(id = 2)]
	#[serde(rename = "number")]
	Number(f64),

	#[buffalo(id = 3)]
	#[serde(rename = "string")]
	String(Arc<str>),

	#[buffalo(id = 4)]
	#[serde(rename = "directory")]
	Directory(Directory),

	#[buffalo(id = 5)]
	#[serde(rename = "file")]
	File(File),

	#[buffalo(id = 6)]
	#[serde(rename = "symlink")]
	Symlink(Symlink),

	#[buffalo(id = 7)]
	#[serde(rename = "dependency")]
	Dependency(Dependency),

	#[buffalo(id = 8)]
	#[serde(rename = "package")]
	Package(Package),

	#[buffalo(id = 9)]
	#[serde(rename = "template")]
	Template(Template),

	#[buffalo(id = 10)]
	#[serde(rename = "placeholder")]
	Placeholder(Placeholder),

	#[buffalo(id = 11)]
	#[serde(rename = "download")]
	Download(Download),

	#[buffalo(id = 12)]
	#[serde(rename = "process")]
	Process(Process),

	#[buffalo(id = 13)]
	#[serde(rename = "target")]
	Target(Target),

	#[buffalo(id = 14)]
	#[serde(rename = "array")]
	Array(Array),

	#[buffalo(id = 15)]
	#[serde(rename = "map")]
	Map(Map),
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
pub struct Package {
	#[buffalo(id = 0)]
	pub source: Hash,

	#[buffalo(id = 1)]
	pub dependencies: BTreeMap<Arc<str>, Hash>,
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
pub struct Template {
	#[buffalo(id = 0)]
	pub components: Vec<Hash>,
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
pub struct Placeholder {
	#[buffalo(id = 0)]
	pub name: String,
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
pub struct Download {
	#[buffalo(id = 0)]
	pub url: Url,

	#[buffalo(id = 1)]
	pub checksum: Option<Checksum>,

	#[buffalo(id = 2)]
	pub unpack: bool,
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
#[serde(rename_all = "camelCase")]
pub struct Process {
	#[buffalo(id = 0)]
	pub system: System,

	#[buffalo(id = 1)]
	pub working_directory: Hash,

	#[buffalo(id = 2)]
	pub env: Hash,

	#[buffalo(id = 3)]
	pub command: Hash,

	#[buffalo(id = 4)]
	pub args: Hash,

	#[buffalo(id = 5)]
	#[serde(default)]
	pub network: bool,

	#[buffalo(id = 6)]
	#[serde(default)]
	pub checksum: Option<Checksum>,

	#[buffalo(id = 7)]
	#[serde(default, rename = "unsafe")]
	pub is_unsafe: bool,
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
pub struct Target {
	#[buffalo(id = 0)]
	pub package: Hash,

	#[buffalo(id = 1)]
	pub name: String,

	#[buffalo(id = 2)]
	pub args: Hash,
}

pub type Array = Vec<Hash>;

pub type Map = BTreeMap<Arc<str>, Hash>;

impl Expression {
	pub fn deserialize<R>(mut reader: R) -> Result<Expression>
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

	pub fn deserialize_from_slice(slice: &[u8]) -> Result<Expression> {
		Expression::deserialize(slice)
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

impl Expression {
	#[must_use]
	pub fn as_null(&self) -> Option<&()> {
		if let Expression::Null(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_bool(&self) -> Option<&bool> {
		if let Expression::Bool(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_number(&self) -> Option<&f64> {
		if let Expression::Number(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_string(&self) -> Option<&Arc<str>> {
		if let Expression::String(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_directory(&self) -> Option<&Directory> {
		if let Expression::Directory(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_file(&self) -> Option<&File> {
		if let Expression::File(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_symlink(&self) -> Option<&Symlink> {
		if let Expression::Symlink(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_dependency(&self) -> Option<&Dependency> {
		if let Expression::Dependency(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_package(&self) -> Option<&Package> {
		if let Expression::Package(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_template(&self) -> Option<&Template> {
		if let Expression::Template(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_placeholder(&self) -> Option<&Placeholder> {
		if let Expression::Placeholder(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_download(&self) -> Option<&Download> {
		if let Expression::Download(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_process(&self) -> Option<&Process> {
		if let Expression::Process(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_target(&self) -> Option<&Target> {
		if let Expression::Target(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_array(&self) -> Option<&Array> {
		if let Expression::Array(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_map(&self) -> Option<&Map> {
		if let Expression::Map(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

impl Expression {
	#[must_use]
	pub fn into_null(self) -> Option<()> {
		if let Expression::Null(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_bool(self) -> Option<bool> {
		if let Expression::Bool(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_number(self) -> Option<f64> {
		if let Expression::Number(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_string(self) -> Option<Arc<str>> {
		if let Expression::String(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_directory(self) -> Option<Directory> {
		if let Expression::Directory(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_file(self) -> Option<File> {
		if let Expression::File(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_symlink(self) -> Option<Symlink> {
		if let Expression::Symlink(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_dependency(self) -> Option<Dependency> {
		if let Expression::Dependency(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_package(self) -> Option<Package> {
		if let Expression::Package(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_template(self) -> Option<Template> {
		if let Expression::Template(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_placeholder(self) -> Option<Placeholder> {
		if let Expression::Placeholder(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_download(self) -> Option<Download> {
		if let Expression::Download(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_process(self) -> Option<Process> {
		if let Expression::Process(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_target(self) -> Option<Target> {
		if let Expression::Target(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_array(self) -> Option<Array> {
		if let Expression::Array(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_map(self) -> Option<Map> {
		if let Expression::Map(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AddExpressionOutcome {
	Added { hash: Hash },
	DirectoryMissingEntries { entries: Vec<(String, Hash)> },
	FileMissingBlob { blob_hash: Hash },
	DependencyMissing { hash: Hash },
	MissingExpressions { hashes: Vec<Hash> },
}
