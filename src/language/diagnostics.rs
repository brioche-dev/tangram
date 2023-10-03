use super::{service, Location, Module, Service};
use crate::{return_error, Result};

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

impl Module {
	pub async fn diagnostics(service: &Service) -> Result<Vec<Diagnostic>> {
		// Create the language service request.
		let request = service::Request::Diagnostics(service::diagnostics::Request {});

		// Perform the language service request.
		let response = service.request(request).await?;

		// Get the response.
		let service::Response::Diagnostics(response) = response else {
			return_error!("Unexpected response type.")
		};

		// Get the result the response.
		let service::diagnostics::Response { diagnostics } = response;

		Ok(diagnostics)
	}
}
