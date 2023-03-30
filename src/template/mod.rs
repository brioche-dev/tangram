use crate::{artifact, placeholder::Placeholder};

mod references;
mod render;
mod unrender;

#[derive(
	Clone,
	Debug,
	Default,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Template {
	#[buffalo(id = 0)]
	pub components: Vec<Component>,
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
#[serde(tag = "kind", content = "value")]
pub enum Component {
	#[buffalo(id = 0)]
	#[serde(rename = "string")]
	String(String),

	#[buffalo(id = 1)]
	#[serde(rename = "artifact")]
	Artifact(artifact::Hash),

	#[buffalo(id = 2)]
	#[serde(rename = "placeholder")]
	Placeholder(Placeholder),
}

impl Template {
	#[must_use]
	pub fn new(components: Vec<Component>) -> Self {
		Self { components }
	}
}

impl From<String> for Template {
	fn from(value: String) -> Self {
		Template {
			components: vec![Component::String(value)],
		}
	}
}

impl From<&str> for Template {
	fn from(value: &str) -> Self {
		value.to_owned().into()
	}
}

impl FromIterator<Component> for Template {
	fn from_iter<I: IntoIterator<Item = Component>>(iter: I) -> Self {
		Template {
			components: iter.into_iter().collect(),
		}
	}
}
