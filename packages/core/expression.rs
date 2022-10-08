use crate::{hash::Hash, system::System};
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
	#[serde(rename = "package")]
	Package(Package),
	#[serde(rename = "template")]
	Template(Template),
	#[serde(rename = "js")]
	Js(Js),
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

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Artifact {
	pub root: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Directory {
	pub entries: BTreeMap<String, Hash>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct File {
	pub blob: Hash,
	pub executable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Symlink {
	pub target: Utf8PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Dependency {
	pub artifact: Hash,
	pub path: Option<Utf8PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Package {
	pub source: Hash,
	pub dependencies: BTreeMap<Arc<str>, Hash>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Template {
	pub components: Vec<Hash>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Js {
	pub package: Hash,
	pub path: Utf8PathBuf,
	pub name: String,
	pub args: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Fetch {
	pub url: Url,
	pub hash: Option<Hash>,
	pub unpack: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Process {
	pub system: System,
	pub env: Hash,
	pub command: Hash,
	pub args: Hash,
	pub hash: Option<Hash>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Target {
	pub package: Hash,
	pub name: String,
	pub args: Hash,
}

pub type Array = Vec<Hash>;

pub type Map = BTreeMap<Arc<str>, Hash>;

impl Expression {
	#[must_use]
	pub fn is_null(&self) -> bool {
		matches!(self, Expression::Null)
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
	pub fn as_artifact(&self) -> Option<&Artifact> {
		if let Expression::Artifact(v) = self {
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
	pub fn as_fetch(&self) -> Option<&Fetch> {
		if let Expression::Fetch(v) = self {
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
	pub fn into_artifact(self) -> Option<Artifact> {
		if let Expression::Artifact(v) = self {
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
	pub fn into_fetch(self) -> Option<Fetch> {
		if let Expression::Fetch(v) = self {
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

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AddExpressionOutcome {
	Added { hash: Hash },
	DirectoryMissingEntries { entries: Vec<(String, Hash)> },
	FileMissingBlob { blob_hash: Hash },
	DependencyMissing { hash: Hash },
	MissingExpressions { hashes: Vec<Hash> },
}
