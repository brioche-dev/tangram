use crate::{language::doc::Doc, module};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
	pub module_identifier: module::Identifier,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	pub doc: Doc,
}
