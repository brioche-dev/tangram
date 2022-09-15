use crate::{hash::Hash, lockfile::Lockfile};
use camino::{Utf8Path, Utf8PathBuf};
use derive_more::Display;
use std::{collections::BTreeMap, sync::Arc};
use url::Url;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Expression {
	Null,
	Bool(bool),
	Number(f64),
	String(Arc<str>),
	Artifact(Artifact),
	Directory(Directory),
	File(File),
	Symlink(Symlink),
	Dependency(Dependency),
	Path(Path),
	Template(Template),
	Fetch(Fetch),
	Process(Process),
	Target(Target),
	Array(Array),
	Map(Map),
}

#[derive(Copy, Clone, Debug, Display, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(from = "ArtifactSerde", into = "ArtifactSerde")]
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
	pub artifact: Artifact,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(from = "PathSerde", into = "PathSerde")]
pub struct Path {
	pub artifact: Hash,
	pub path: Option<Arc<Utf8Path>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(from = "TemplateSerde", into = "TemplateSerde")]
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
	pub package: Artifact,
	pub name: String,
	pub args: Vec<Hash>,
}

pub type Array = Vec<Hash>;

pub type Map = BTreeMap<Arc<str>, Hash>;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_tangram")]
enum ArtifactSerde {
	#[serde(rename = "artifact")]
	Artifact { hash: Hash },
}

impl From<Artifact> for ArtifactSerde {
	fn from(value: Artifact) -> ArtifactSerde {
		ArtifactSerde::Artifact { hash: value.hash }
	}
}

impl From<ArtifactSerde> for Artifact {
	fn from(value: ArtifactSerde) -> Self {
		let ArtifactSerde::Artifact { hash } = value;
		Artifact { hash }
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_tangram")]
enum DirectorySerde {
	#[serde(rename = "directory")]
	Directory { entries: BTreeMap<String, Hash> },
}

impl From<Directory> for DirectorySerde {
	fn from(value: Directory) -> DirectorySerde {
		DirectorySerde::Directory {
			entries: value.entries,
		}
	}
}

impl From<DirectorySerde> for Directory {
	fn from(value: DirectorySerde) -> Directory {
		let DirectorySerde::Directory { entries } = value;
		Directory { entries }
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_tangram")]
enum FileSerde {
	#[serde(rename = "file")]
	File { blob_hash: Hash, executable: bool },
}

impl From<File> for FileSerde {
	fn from(value: File) -> FileSerde {
		FileSerde::File {
			blob_hash: value.blob_hash,
			executable: value.executable,
		}
	}
}

impl From<FileSerde> for File {
	fn from(value: FileSerde) -> File {
		let FileSerde::File {
			blob_hash,
			executable,
		} = value;
		File {
			blob_hash,
			executable,
		}
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_tangram")]
enum SymlinkSerde {
	#[serde(rename = "symlink")]
	Symlink { target: Utf8PathBuf },
}

impl From<Symlink> for SymlinkSerde {
	fn from(value: Symlink) -> SymlinkSerde {
		SymlinkSerde::Symlink {
			target: value.target,
		}
	}
}

impl From<SymlinkSerde> for Symlink {
	fn from(value: SymlinkSerde) -> Symlink {
		let SymlinkSerde::Symlink { target } = value;
		Symlink { target }
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_tangram")]
enum DependencySerde {
	#[serde(rename = "dependency")]
	Dependency { artifact: Artifact },
}

impl From<Dependency> for DependencySerde {
	fn from(value: Dependency) -> DependencySerde {
		DependencySerde::Dependency {
			artifact: value.artifact,
		}
	}
}

impl From<DependencySerde> for Dependency {
	fn from(value: DependencySerde) -> Dependency {
		let DependencySerde::Dependency { artifact } = value;
		Dependency { artifact }
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_tangram")]
enum PathSerde {
	#[serde(rename = "path")]
	Path {
		artifact: Hash,
		path: Option<Utf8PathBuf>,
	},
}

impl From<Path> for PathSerde {
	fn from(value: Path) -> PathSerde {
		PathSerde::Path {
			artifact: value.artifact,
			path: value.path.map(|path| path.as_ref().to_owned()),
		}
	}
}

impl From<PathSerde> for Path {
	fn from(value: PathSerde) -> Self {
		let (artifact, path) = match value {
			PathSerde::Path { artifact, path } => (artifact, path),
		};
		Path {
			artifact,
			path: path.map(Into::into),
		}
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_tangram")]
enum TemplateSerde {
	#[serde(rename = "template")]
	Template { components: Vec<Hash> },
}

impl From<Template> for TemplateSerde {
	fn from(value: Template) -> TemplateSerde {
		TemplateSerde::Template {
			components: value.components,
		}
	}
}

impl From<TemplateSerde> for Template {
	fn from(value: TemplateSerde) -> Self {
		let components = match value {
			TemplateSerde::Template { components } => components,
		};
		Template { components }
	}
}

impl Expression {
	#[must_use]
	pub fn hash(&self) -> Hash {
		Hash::new(serde_json::to_vec(self).unwrap())
	}
}
