use super::{service, Location};
use crate::{
	error::{return_error, Result},
	Instance,
};
use std::sync::Arc;

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

impl Instance {
	pub async fn diagnostics(self: &Arc<Self>) -> Result<Vec<Diagnostic>> {
		// Create the language service request.
		let request = service::Request::Diagnostics(service::diagnostics::Request {});

		// Handle the language service request.
		let response = self.handle_language_service_request(request).await?;

		// Get the response.
		let service::Response::Diagnostics(response) = response else { return_error!("Unexpected response type.") };

		// Get the result the response.
		let service::diagnostics::Response { diagnostics } = response;

		Ok(diagnostics)
	}
}
