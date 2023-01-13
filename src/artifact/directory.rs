use super::ArtifactHash;
use std::collections::BTreeMap;

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
pub struct Directory {
	#[buffalo(id = 0)]
	pub entries: BTreeMap<String, ArtifactHash>,
}
