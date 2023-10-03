use super::{service, Module, Position, Service};
use crate::{return_error, Result};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
	pub name: String,
}

impl Module {
	pub async fn completion(
		&self,
		service: &Service,
		position: Position,
	) -> Result<Option<Vec<Entry>>> {
		// Create the language service request.
		let request = service::Request::Completion(service::completion::Request {
			module: self.clone(),
			position,
		});

		// Perform the language service request.
		let response = service.request(request).await?;

		// Get the response.
		let service::Response::Completion(response) = response else {
			return_error!("Unexpected response type.")
		};

		Ok(response.entries)
	}
}
