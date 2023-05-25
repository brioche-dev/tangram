use super::{service, location::Location,};
use crate::{
	module::position::Position,
	error::{return_error, Result},
	instance::Instance,
	module::Module,
};
use std::sync::Arc;

impl Module {
	pub async fn references(
		&self,
		tg: &Arc<Instance>,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the language service request.
		let request = service::Request::References(service::references::Request {
			module: self.clone(),
			position,
		});

		// Handle the language service request.
		let response = tg.handle_language_service_request(request).await?;

		// Get the response.
		let service::Response::References(response) = response else { return_error!("Unexpected response type.") };

		Ok(response.locations)
	}
}
