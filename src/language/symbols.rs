use super::{service, Module, Service};
use crate::{return_error, Result};

impl Module {
	pub async fn symbols(
		&self,
		service: &Service,
	) -> Result<Option<Vec<service::symbols::Symbol>>> {
		// Create the language service request.
		let request = service::Request::Symbols(service::symbols::Request {
			module: self.clone(),
		});

		// Perform the language service request.
		let response = service.request(request).await?;

		// Get the response.
		let service::Response::Symbols(response) = response else {
			return_error!("Unexpected response type.")
		};

		Ok(response.symbols)
	}
}
