use super::Server;
use crate::{module::Diagnostic, Module, Result};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
	pub modules: Vec<Module>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	pub diagnostics: Vec<Diagnostic>,
}

impl Server {
	/// Get all diagnostics for the provided modules.
	pub async fn check(&self, modules: Vec<Module>) -> Result<Vec<Diagnostic>> {
		// Create the language service request.
		let request = super::Request::Check(Request { modules });

		// Perform the request.
		let response = self.request(request).await?.unwrap_check();

		Ok(response.diagnostics)
	}
}
