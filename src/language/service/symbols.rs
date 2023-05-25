use crate::{module::range::Range, module};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
	pub module: module::Module,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	pub symbols: Option<Vec<Symbol>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Symbol {
	pub name: String,
	pub detail: Option<String>,
	pub kind: Kind,
	pub tags: Vec<Tag>,
	pub range: Range,
	pub selection_range: Range,
	pub children: Option<Vec<Self>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
	File,
	Module,
	Namespace,
	Package,
	Class,
	Method,
	Property,
	Field,
	Constructor,
	Enum,
	Interface,
	Function,
	Variable,
	Constant,
	String,
	Number,
	Boolean,
	Array,
	Object,
	Key,
	Null,
	EnumMember,
	Event,
	Operator,
	TypeParameter,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tag {
	Deprecated,
}
