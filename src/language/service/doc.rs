use std::collections::BTreeMap;

use crate::{language::doc::Symbol, module};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
	pub module: module::Module,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	pub exports: BTreeMap<String, Symbol>,
}
