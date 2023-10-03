use super::{service, Location, Module, Position, Service};
use crate::{return_error, Result};

impl Module {
	pub async fn references(
		&self,
		service: &Service,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the language service request.
		let request = service::Request::References(service::references::Request {
			module: self.clone(),
			position,
		});

		// Perform the language service request.
		let response = service.request(request).await?;

		// Get the response.
		let service::Response::References(response) = response else {
			return_error!("Unexpected response type.")
		};

		Ok(response.locations)
	}
}
