use crate::artifact::Artifact;
use camino::Utf8PathBuf;
use std::collections::BTreeMap;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Value {
	Null,
	Bool(bool),
	Number(f64),
	String(String),
	Artifact(Artifact),
	Path(Path),
	Template(Template),
	Array(Vec<Value>),
	Map(BTreeMap<String, Value>),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(from = "PathSerde", into = "PathSerde")]
pub struct Path {
	pub artifact: Artifact,
	pub path: Option<Utf8PathBuf>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(from = "TemplateSerde", into = "TemplateSerde")]
pub struct Template {
	pub components: Vec<Value>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_tangram")]
enum PathSerde {
	#[serde(rename = "path")]
	Path {
		artifact: Artifact,
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
	Template { components: Vec<Value> },
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
