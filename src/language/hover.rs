use super::{service, Position};
use crate::{
	error::{return_error, Result},
	module, Instance,
};
use std::sync::Arc;

impl Instance {
	pub async fn hover(
		self: &Arc<Self>,
		module_identifier: module::Identifier,
		position: Position,
	) -> Result<Option<String>> {
		// Create the language service request.
		let request = service::Request::Hover(service::hover::Request {
			module_identifier,
			position,
		});

		// Handle the language service request.
		let response = self.handle_language_service_request(request).await?;

		// Get the response.
		let service::Response::Hover(response) = response else { return_error!("Unexpected response type.") };

		// Get the text from the response.
		let service::hover::Response { text } = response;

		Ok(text)
	}
}
