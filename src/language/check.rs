use super::{service, Diagnostic};
use crate::{
	error::{return_error, Result},
	module, Instance,
};
use std::sync::Arc;

impl Instance {
	/// Get all diagnostics for the provided module identifiers.
	pub async fn check(
		self: &Arc<Self>,
		module_identifiers: Vec<module::Identifier>,
	) -> Result<Vec<Diagnostic>> {
		// Create the language service request.
		let request = service::Request::Check(service::check::Request { module_identifiers });

		// Handle the language service request.
		let response = self.handle_language_service_request(request).await?;

		// Get the response.
		let service::Response::Check(response) = response else { return_error!("Unexpected response type.") };

		Ok(response.diagnostics)
	}
}
