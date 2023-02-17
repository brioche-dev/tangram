use crate::{artifact, path::Path};

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
pub struct Reference {
	#[buffalo(id = 0)]
	#[serde(rename = "artifactHash")]
	pub artifact_hash: artifact::Hash,

	#[buffalo(id = 1)]
	pub path: Option<Path>,
}
