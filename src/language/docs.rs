use super::{service, Module, Service};
use crate::{return_error, Result};

impl Module {
	/// Get the docs for a module.
	pub async fn docs(&self, service: &Service) -> Result<service::docs::Response> {
		// Create the language service request.
		let request = service::Request::Docs(service::docs::Request {
			module: self.clone(),
		});

		// Perform the language service request.
		let response = service.request(request).await?;

		// Get the response.
		let service::Response::Docs(response) = response else {
			return_error!("Unexpected response type.")
		};

		Ok(response)
	}
}
