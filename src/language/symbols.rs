use super::{service, Module};
use crate::{return_error, server::Server, Result};

impl Module {
	pub async fn symbols(&self, server: &Server) -> Result<Option<Vec<service::symbols::Symbol>>> {
		// Create the language service request.
		let request = service::Request::Symbols(service::symbols::Request {
			module: self.clone(),
		});

		// Handle the language service request.
		let response = server.handle_language_service_request(request).await?;

		// Get the response.
		let service::Response::Symbols(response) = response else {
			return_error!("Unexpected response type.")
		};

		Ok(response.symbols)
	}
}
