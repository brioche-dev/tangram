use super::{service, Module, Position, Service};
use crate::{return_error, Result};

impl Module {
	pub async fn hover(&self, service: &Service, position: Position) -> Result<Option<String>> {
		// Create the language service request.
		let request = service::Request::Hover(service::hover::Request {
			module: self.clone(),
			position,
		});

		// Perform the language service request.
		let response = service.request(request).await?;

		// Get the response.
		let service::Response::Hover(response) = response else {
			return_error!("Unexpected response type.")
		};

		// Get the text from the response.
		let service::hover::Response { text } = response;

		Ok(text)
	}
}
