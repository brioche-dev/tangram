use super::{service, Location, Position};
use crate::{
	error::{return_error, Result},
	module, Instance,
};
use std::sync::Arc;

impl Instance {
	pub async fn definition(
		self: &Arc<Self>,
		module_identifier: module::Identifier,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the language service request.
		let request = service::Request::Definition(service::definition::Request {
			module_identifier,
			position,
		});

		// Send the language service request and receive the response.
		let response = self.language_service_request(request).await?;

		// Get the response.
		let service::Response::Definition(response) = response else { return_error!("Unexpected response type.") };

		Ok(response.locations)
	}
}
