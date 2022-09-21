use crate::{hash::Hash, lockfile::Lockfile};
use camino::Utf8PathBuf;
use derive_more::Display;
use std::{collections::BTreeMap, sync::Arc};
use url::Url;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Expression {
	#[serde(rename = "null")]
	Null,
	#[serde(rename = "bool")]
	Bool(bool),
	#[serde(rename = "number")]
	Number(f64),
	#[serde(rename = "string")]
	String(Arc<str>),
	#[serde(rename = "artifact")]
	Artifact(Artifact),
	#[serde(rename = "directory")]
	Directory(Directory),
	#[serde(rename = "file")]
	File(File),
	#[serde(rename = "symlink")]
	Symlink(Symlink),
	#[serde(rename = "dependency")]
	Dependency(Dependency),
	#[serde(rename = "path")]
	Path(Path),
	#[serde(rename = "template")]
	Template(Template),
	#[serde(rename = "fetch")]
	Fetch(Fetch),
	#[serde(rename = "process")]
	Process(Process),
	#[serde(rename = "target")]
	Target(Target),
	#[serde(rename = "array")]
	Array(Array),
	#[serde(rename = "map")]
	Map(Map),
}

#[derive(Copy, Clone, Debug, Display, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Artifact {
	pub hash: Hash,
}

/// An expression representing a directory.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Directory {
	pub entries: BTreeMap<String, Hash>,
}

/// An expression representing a file.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct File {
	pub blob_hash: Hash,
	pub executable: bool,
}

/// An expression representing a symbolic link.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Symlink {
	pub target: Utf8PathBuf,
}

/// An expression representing a dependency on another artifact.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Dependency {
	pub artifact: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Path {
	pub artifact: Hash,
	pub path: Option<Utf8PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Template {
	pub components: Vec<Hash>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Fetch {
	pub url: Url,
	pub hash: Option<Hash>,
	pub unpack: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "system")]
pub enum Process {
	#[serde(rename = "amd64_linux")]
	Amd64Linux(UnixProcess),
	#[serde(rename = "amd64_macos")]
	Amd64Macos(UnixProcess),
	#[serde(rename = "arm64_linux")]
	Arm64Linux(UnixProcess),
	#[serde(rename = "arm64_macos")]
	Arm64Macos(UnixProcess),
	#[serde(rename = "js")]
	Js(JsProcess),
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UnixProcess {
	pub env: Hash,
	pub command: Hash,
	pub args: Hash,
	pub outputs: BTreeMap<String, UnixProcessOutput>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UnixProcessOutput {
	#[serde(default)]
	pub dependencies: BTreeMap<Utf8PathBuf, Hash>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JsProcess {
	pub lockfile: Option<Lockfile>,
	pub module: Hash,
	pub export: String,
	pub args: Vec<Hash>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Target {
	pub lockfile: Option<Lockfile>,
	pub package: Hash,
	pub name: String,
	pub args: Vec<Hash>,
}

pub type Array = Vec<Hash>;

pub type Map = BTreeMap<Arc<str>, Hash>;

impl Expression {
	#[must_use]
	pub fn hash(&self) -> Hash {
		Hash::new(serde_json::to_vec(self).unwrap())
	}
}
