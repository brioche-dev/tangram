#![allow(clippy::module_name_repetitions)]

use crate::compiler::{CompletionEntry, Diagnostic, Location, ModuleIdentifier, Position};
use std::collections::BTreeMap;

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", content = "request", rename_all = "snake_case")]
pub enum Request {
	Check(CheckRequest),
	Rename(RenameRequest),
	Diagnostics(DiagnosticsRequest),
	Definition(DefintionRequest),
	Hover(HoverRequest),
	References(ReferencesRequest),
	Completion(CompletionRequest),
	Transpile(TranspileRequest),
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", content = "response", rename_all = "snake_case")]
pub enum Response {
	Check(CheckResponse),
	Rename(RenameLocationsResponse),
	Diagnostics(DiagnosticsResponse),
	Hover(HoverResponse),
	References(ReferencesResponse),
	Definition(DefinitionResponse),
	Completion(CompletionResponse),
	Transpile(TranspileResponse),
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckRequest {
	pub module_identifiers: Vec<ModuleIdentifier>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResponse {
	pub diagnostics: BTreeMap<ModuleIdentifier, Vec<Diagnostic>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameRequest {
	pub module_identifier: ModuleIdentifier,
	pub position: Position,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameLocationsResponse {
	pub locations: Option<Vec<Location>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsRequest {}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsResponse {
	pub diagnostics: BTreeMap<ModuleIdentifier, Vec<Diagnostic>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HoverRequest {
	pub module_identifier: ModuleIdentifier,
	pub position: Position,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoverResponse {
	pub text: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferencesRequest {
	pub module_identifier: ModuleIdentifier,
	pub position: Position,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferencesResponse {
	pub locations: Option<Vec<Location>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefintionRequest {
	pub module_identifier: ModuleIdentifier,
	pub position: Position,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionResponse {
	pub locations: Option<Vec<Location>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionRequest {
	pub module_identifier: ModuleIdentifier,
	pub position: Position,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionResponse {
	pub entries: Option<Vec<CompletionEntry>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranspileRequest {
	pub text: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranspileResponse {
	pub output_text: String,
	pub source_map_text: String,
}
