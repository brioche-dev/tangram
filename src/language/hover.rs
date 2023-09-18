use super::service;
use crate::{
	error::{return_error, Result},
	module::position::Position,
	module::Module,
	server::Server,
};

impl Module {
	pub async fn hover(&self, server: &Server, position: Position) -> Result<Option<String>> {
		// Create the language service request.
		let request = service::Request::Hover(service::hover::Request {
			module: self.clone(),
			position,
		});

		// Handle the language service request.
		let response = server.handle_language_service_request(request).await?;

		// Get the response.
		let service::Response::Hover(response) = response else { return_error!("Unexpected response type.") };

		// Get the text from the response.
		let service::hover::Response { text } = response;

		Ok(text)
	}
}
