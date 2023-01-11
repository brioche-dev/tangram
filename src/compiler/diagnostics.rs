use super::{
	request::{DiagnosticsRequest, DiagnosticsResponse, Request, Response},
	Compiler, Diagnostic, ModuleIdentifier,
};
use anyhow::{bail, Result};
use std::collections::BTreeMap;

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
