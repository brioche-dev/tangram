use crate::{language::Diagnostic, module};
use std::collections::BTreeMap;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
	pub module_identifiers: Vec<module::Identifier>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	pub diagnostics: BTreeMap<module::Identifier, Vec<Diagnostic>>,
}
