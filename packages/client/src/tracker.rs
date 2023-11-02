use crate::artifact;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Tracker {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub artifact: Option<artifact::Id>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub package: Option<artifact::Id>,
}
