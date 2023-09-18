use super::service;
use crate::{
	error::{return_error, Result},
	module::position::Position,
	module::Module,
	server::Server,
};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
	pub name: String,
}

impl Module {
	pub async fn completion(
		&self,
		server: &Server,
		position: Position,
	) -> Result<Option<Vec<Entry>>> {
		// Create the language service request.
		let request = service::Request::Completion(service::completion::Request {
			module: self.clone(),
			position,
		});

		// Handle the language service request.
		let response = server.handle_language_service_request(request).await?;

		// Get the response.
		let service::Response::Completion(response) = response else { return_error!("Unexpected response type.") };

		Ok(response.entries)
	}
}
