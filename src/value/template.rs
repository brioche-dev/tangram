use super::Placeholder;
use crate::artifact::ArtifactHash;

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
	pub components: Vec<TemplateComponent>,
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
#[serde(tag = "type", content = "value")]
pub enum TemplateComponent {
	#[buffalo(id = 0)]
	#[serde(rename = "string")]
	String(String),

	#[buffalo(id = 1)]
	#[serde(rename = "artifact")]
	Artifact(ArtifactHash),

	#[buffalo(id = 2)]
	#[serde(rename = "placeholder")]
	Placeholder(Placeholder),
}
