use crate::{hash::Hash, lockfile::Lockfile};
use camino::Utf8PathBuf;
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
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
	pub hash: Hash,
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
	pub artifact: Hash,
	pub path: Option<Utf8PathBuf>,
	pub export: String,
	pub args: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Target {
	pub lockfile: Option<Lockfile>,
	pub package: Hash,
	pub name: String,
	pub args: Hash,
}

pub type Array = Vec<Hash>;

pub type Map = BTreeMap<Arc<str>, Hash>;

impl Expression {
	#[must_use]
	pub fn is_null(&self) -> bool {
		matches!(self, Self::Null)
	}

	pub fn as_bool(&self) -> Option<&bool> {
		if let Self::Bool(v) = self {
			Some(v)
		} else {
			None
		}
	}

	pub fn as_number(&self) -> Option<&f64> {
		if let Self::Number(v) = self {
			Some(v)
		} else {
			None
		}
	}

	pub fn as_string(&self) -> Option<&Arc<str>> {
		if let Self::String(v) = self {
			Some(v)
		} else {
			None
		}
	}

	pub fn as_artifact(&self) -> Option<&Artifact> {
		if let Self::Artifact(v) = self {
			Some(v)
		} else {
			None
		}
	}

	pub fn as_directory(&self) -> Option<&Directory> {
		if let Self::Directory(v) = self {
			Some(v)
		} else {
			None
		}
	}

	pub fn as_file(&self) -> Option<&File> {
		if let Self::File(v) = self {
			Some(v)
		} else {
			None
		}
	}

	pub fn as_symlink(&self) -> Option<&Symlink> {
		if let Self::Symlink(v) = self {
			Some(v)
		} else {
			None
		}
	}

	pub fn as_dependency(&self) -> Option<&Dependency> {
		if let Self::Dependency(v) = self {
			Some(v)
		} else {
			None
		}
	}

	pub fn as_template(&self) -> Option<&Template> {
		if let Self::Template(v) = self {
			Some(v)
		} else {
			None
		}
	}

	pub fn as_fetch(&self) -> Option<&Fetch> {
		if let Self::Fetch(v) = self {
			Some(v)
		} else {
			None
		}
	}

	pub fn as_process(&self) -> Option<&Process> {
		if let Self::Process(v) = self {
			Some(v)
		} else {
			None
		}
	}

	pub fn as_target(&self) -> Option<&Target> {
		if let Self::Target(v) = self {
			Some(v)
		} else {
			None
		}
	}

	pub fn as_array(&self) -> Option<&Array> {
		if let Self::Array(v) = self {
			Some(v)
		} else {
			None
		}
	}

	pub fn as_map(&self) -> Option<&Map> {
		if let Self::Map(v) = self {
			Some(v)
		} else {
			None
		}
	}
}
