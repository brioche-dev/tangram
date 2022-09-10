use crate::{artifact::Artifact, hash, lockfile::Lockfile};
use camino::{Utf8Path, Utf8PathBuf};
use derive_more::{Display, FromStr};
use std::{collections::BTreeMap, sync::Arc};
use url::Url;

#[derive(
	Clone, Copy, Debug, Display, Eq, FromStr, Hash, PartialEq, serde::Deserialize, serde::Serialize,
)]
pub struct Hash(pub hash::Hash);

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Expression {
	Null,
	Bool(bool),
	Number(f64),
	String(Arc<str>),
	Artifact(Artifact),
	Path(Path),
	Template(Template),
	Fetch(Fetch),
	Process(Process),
	Target(Target),
	Array(Vec<Expression>),
	Map(BTreeMap<Arc<str>, Expression>),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(from = "PathSerde", into = "PathSerde")]
pub struct Path {
	pub artifact: Box<Expression>,
	pub path: Option<Arc<Utf8Path>>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(from = "TemplateSerde", into = "TemplateSerde")]
pub struct Template {
	pub components: Vec<Expression>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Fetch {
	pub url: Url,
	pub hash: Option<hash::Hash>,
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
	#[serde(default = "default_map")]
	pub env: Box<Expression>,
	pub command: Box<Expression>,
	#[serde(default = "default_array")]
	pub args: Box<Expression>,
	#[serde(default = "default_outputs")]
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
		Hash(hash::Hash::new(serde_json::to_vec(self).unwrap()))
	}
}

fn default_array() -> Box<Expression> {
	Box::new(Expression::Array(Vec::default()))
}

fn default_map() -> Box<Expression> {
	Box::new(Expression::Map(BTreeMap::default()))
}

fn default_outputs() -> BTreeMap<String, Output> {
	[(
		"out".into(),
		Output {
			dependencies: BTreeMap::default(),
		},
	)]
	.into()
}
