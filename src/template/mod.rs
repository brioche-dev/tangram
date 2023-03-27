use crate::{artifact, placeholder::Placeholder};

mod references;
mod render;
mod unrender;

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
