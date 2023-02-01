use super::ModuleIdentifier;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
	pub location: Option<Location>,
	pub severity: Severity,
	pub message: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
	Error,
	Warning,
	Information,
	Hint,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
	pub module_identifier: ModuleIdentifier,
	pub range: Range,
}

/// A `Range` represents a range in a string, such as a text editor selection. The end is exclusive. This type maps cleanly to the `Range` type in the Language Server Protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Range {
	pub start: Position,
	pub end: Position,
}

/// A `Position` represents a position in a string, indexed by a line and character offset (both zero-indexed). This type maps cleanly to the `Position` type in the Language Server Protocol. For maximum compatibility with the Language Server Protocol, character offsets use UTF-16 code units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
	pub line: u32,
	pub character: u32,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionEntry {
	pub name: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranspileOutput {
	pub transpiled: String,
	pub source_map: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct TextEdit {
	pub range: Range,
	pub new_text: String,
}
