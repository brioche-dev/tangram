use super::{
	request::{CheckRequest, Request, Response},
	Compiler, Diagnostic, ModuleIdentifier,
};
use anyhow::{bail, Result};
use std::collections::BTreeMap;

impl Compiler {
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

		// Get the result from the response.
		let diagnostics = response.diagnostics;

		Ok(diagnostics)
	}
}
