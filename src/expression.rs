use crate::{artifact::Artifact, hash::Hash, lockfile::Lockfile};
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
	pub path: Option<Utf8PathBuf>,
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UnixProcess {
	pub env: Box<Expression>,
	pub cwd: Box<Expression>,
	pub command: Box<Expression>,
	pub args: Box<Expression>,
	pub outputs: BTreeMap<String, Output>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Output {
	#[serde(default)]
	pub dependencies: BTreeMap<Utf8PathBuf, Box<Expression>>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct JsProcess {
	pub lockfile: Option<Lockfile>,
	pub module: Box<Expression>,
	pub export: String,
	pub args: Vec<Expression>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Target {
	pub lockfile: Option<Lockfile>,
	pub package: Artifact,
	pub name: String,
	pub args: Vec<Expression>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_tangram")]
enum PathSerde {
	#[serde(rename = "path")]
	Path {
		artifact: Box<Expression>,
		path: Option<Utf8PathBuf>,
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

impl Expression {
	#[must_use]
	pub fn hash(&self) -> Hash {
		Hash::new(serde_json::to_vec(self).unwrap())
	}
}
