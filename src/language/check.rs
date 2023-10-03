use super::{service, Diagnostic, Module, Service};
use crate::{return_error, Result};

impl Module {
	/// Get all diagnostics for the provided modules.
	pub async fn check(service: &Service, modules: Vec<Module>) -> Result<Vec<Diagnostic>> {
		// Create the language service request.
		let request = service::Request::Check(service::check::Request { modules });

		// Perform the language service request.
		let response = service.request(request).await?;

		// Get the response.
		let service::Response::Check(response) = response else {
			return_error!("Unexpected response type.")
		};

		Ok(response.diagnostics)
	}
}
