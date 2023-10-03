use super::{service, Location, Module, Position, Service};
use crate::{return_error, Result};

impl Module {
	pub async fn rename(
		&self,
		service: &Service,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the language service request.
		let request = service::Request::Rename(service::rename::Request {
			module: self.clone(),
			position,
		});

		// Perform the language service request.
		let response = service.request(request).await?;

		// Get the response.
		let service::Response::Rename(response) = response else {
			return_error!("Unexpected response type.")
		};

		Ok(response.locations)
	}
}
