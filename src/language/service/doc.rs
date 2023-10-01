use crate::language::{docs::Symbol, Module};
use std::collections::BTreeMap;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
	pub module: Module,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	pub exports: BTreeMap<String, Symbol>,
	pub module: Module,
}
