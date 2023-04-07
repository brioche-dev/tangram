use super::{service, Location, Position};
use crate::{
	error::{return_error, Result},
	instance::Instance,
	module::Module,
};
use std::sync::Arc;

impl Module {
	pub async fn rename(
		&self,
		tg: &Arc<Instance>,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the language service request.
		let request = service::Request::Rename(service::rename::Request {
			module: self.clone(),
			position,
		});

		// Handle the language service request.
		let response = tg.handle_language_service_request(request).await?;

		// Get the response.
		let service::Response::Rename(response) = response else { return_error!("Unexpected response type.") };

		Ok(response.locations)
	}
}
