use super::{Diagnostic, ModuleIdentifier, Request, Response};
use crate::Cli;
use anyhow::{bail, Result};
use std::collections::BTreeMap;

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

impl Cli {
	/// Get all diagnostics for a package.
	pub async fn check(
		&self,
		module_identifiers: Vec<ModuleIdentifier>,
	) -> Result<BTreeMap<ModuleIdentifier, Vec<Diagnostic>>> {
		// Create the request.
		let request = Request::Check(CheckRequest { module_identifiers });

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::Check(response) => response,
			_ => bail!("Unexpected response type."),
		};

		Ok(response.diagnostics)
	}
}
