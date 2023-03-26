use super::{service, Location, Position};
use crate::{
	error::{return_error, Result},
	module, Instance,
};
use std::sync::Arc;

impl Instance {
	pub async fn references(
		self: &Arc<Self>,
		module_identifier: module::Identifier,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the language service request.
		let request = service::Request::References(service::references::Request {
			module_identifier,
			position,
		});

		// Handle the language service request.
		let response = self.handle_language_service_request(request).await?;

		// Get the response.
		let service::Response::References(response) = response else { return_error!("Unexpected response type.") };

		Ok(response.locations)
	}
}
