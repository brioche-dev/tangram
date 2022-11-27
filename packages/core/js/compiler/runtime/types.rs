use crate::js::{
	self,
	compiler::types::{CompletionEntry, Diagnostic, Location, Position},
};
use std::collections::BTreeMap;

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", content = "request", rename_all = "snake_case")]
pub enum Request {
	Check(CheckRequest),
	FindRenameLocations(FindRenameLocationsRequest),
	GetDiagnostics(GetDiagnosticsRequest),
	GotoDefinition(GotoDefintionRequest),
	GetHover(GetHoverRequest),
	GetReferences(GetReferencesRequest),
	Completion(CompletionRequest),
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", content = "response", rename_all = "snake_case")]
pub enum Response {
	Check(CheckResponse),
	FindRenameLocations(FindRenameLocationsResponse),
	GetDiagnostics(GetDiagnosticsResponse),
	GetHover(GetHoverResponse),
	GetReferences(GetReferencesResponse),
	GotoDefinition(GotoDefinitionResponse),
	Completion(CompletionResponse),
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckRequest {
	pub urls: Vec<js::Url>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResponse {
	pub diagnostics: BTreeMap<js::Url, Vec<Diagnostic>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindRenameLocationsRequest {
	pub url: js::Url,
	pub position: Position,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindRenameLocationsResponse {
	pub locations: Option<Vec<Location>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDiagnosticsRequest {}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDiagnosticsResponse {
	pub diagnostics: BTreeMap<js::Url, Vec<Diagnostic>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHoverRequest {
	pub url: js::Url,
	pub position: Position,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHoverResponse {
	pub text: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetReferencesRequest {
	pub url: js::Url,
	pub position: Position,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetReferencesResponse {
	pub locations: Option<Vec<Location>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GotoDefintionRequest {
	pub url: js::Url,
	pub position: Position,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GotoDefinitionResponse {
	pub locations: Option<Vec<Location>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionRequest {
	pub url: js::Url,
	pub position: Position,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionResponse {
	pub entries: Option<Vec<CompletionEntry>>,
}
