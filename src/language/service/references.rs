use crate::{
	language::{Location, Position},
	module,
};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
	pub module_identifier: module::Identifier,
	pub position: Position,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	pub locations: Option<Vec<Location>>,
}
