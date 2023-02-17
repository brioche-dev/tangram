use super::Range;
use crate::module;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
	pub module_identifier: module::Identifier,
	pub range: Range,
}
