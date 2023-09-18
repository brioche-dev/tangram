use super::{location::Location, service};
use crate::{
	error::{return_error, Result},
	module::position::Position,
	module::Module,
	server::Server,
};

impl Module {
	pub async fn rename(
		&self,
		server: &Server,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the language service request.
		let request = service::Request::Rename(service::rename::Request {
			module: self.clone(),
			position,
		});

		// Handle the language service request.
		let response = server.handle_language_service_request(request).await?;

		// Get the response.
		let service::Response::Rename(response) = response else { return_error!("Unexpected response type.") };

		Ok(response.locations)
	}
}
