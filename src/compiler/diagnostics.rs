use super::{Compiler, Diagnostic, ModuleIdentifier, Request, Response};
use anyhow::{bail, Result};
use std::collections::BTreeMap;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsRequest {}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsResponse {
	pub diagnostics: BTreeMap<ModuleIdentifier, Vec<Diagnostic>>,
}

impl Compiler {
	pub async fn diagnostics(&self) -> Result<BTreeMap<ModuleIdentifier, Vec<Diagnostic>>> {
		// Create the request.
		let request = Request::Diagnostics(DiagnosticsRequest {});

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::Diagnostics(response) => response,
			_ => bail!("Unexpected response type."),
		};

		// Get the result the response.
		let DiagnosticsResponse { diagnostics } = response;

		Ok(diagnostics)
	}
}
