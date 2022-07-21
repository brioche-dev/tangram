use crate::{
	artifact::{Artifact, ArtifactHash},
	hash::Hash,
	lockfile::Lockfile,
};
use camino::Utf8PathBuf;
use std::collections::BTreeMap;
use url::Url;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[allow(clippy::module_name_repetitions)]
pub struct ExpressionHash(Hash);

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Expression {
	Null,
	Bool(bool),
	Number(f64),
	String(String),
	Artifact(Artifact),
	Path(Path),
	Template(Template),
	Fetch(Fetch),
	Process(Process),
	Target(Target),
	Array(Vec<Expression>),
	Map(BTreeMap<String, Expression>),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(from = "PathSerde", into = "PathSerde")]
pub struct Path {
	pub artifact: Box<Expression>,
	pub path: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(from = "TemplateSerde", into = "TemplateSerde")]
pub struct Template {
	pub components: Vec<Expression>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Fetch {
	pub url: Url,
	pub hash: Option<Hash>,
	pub unpack: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "system")]
pub enum Process {
	Amd64Linux(UnixProcess),
	Amd64Macos(UnixProcess),
	Arm64Linux(UnixProcess),
	Arm64Macos(UnixProcess),
	Js(JsProcess),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UnixProcess {
	pub command: Box<Expression>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub args: Vec<Expression>,
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub env: BTreeMap<String, Expression>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub cwd: Option<Box<Expression>>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct JsProcess {
	pub package: Box<Expression>,
	pub path: Utf8PathBuf,
	pub lockfile: Lockfile,
	pub module: Box<Expression>,
	pub export: String,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub args: Vec<Expression>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Target {
	pub artifact_hash: ArtifactHash,
	pub lockfile: Option<Lockfile>,
	pub export: String,
	pub args: Vec<Expression>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_tangram")]
enum PathSerde {
	#[serde(rename = "path")]
	Path {
		artifact: Box<Expression>,
		path: Option<String>,
	},
}

impl From<Path> for PathSerde {
	fn from(value: Path) -> PathSerde {
		PathSerde::Path {
			artifact: value.artifact,
			path: value.path,
		}
	}
}

impl From<PathSerde> for Path {
	fn from(value: PathSerde) -> Self {
		let (artifact, path) = match value {
			PathSerde::Path { artifact, path } => (artifact, path),
		};
		Path { artifact, path }
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_tangram")]
enum TemplateSerde {
	#[serde(rename = "template")]
	Template { components: Vec<Expression> },
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
