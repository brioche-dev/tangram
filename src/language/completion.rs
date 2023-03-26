use super::{service, Position};
use crate::{
	error::{return_error, Result},
	module, Instance,
};
use std::sync::Arc;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
	pub name: String,
}

impl Instance {
	pub async fn completion(
		self: &Arc<Self>,
		module_identifier: module::Identifier,
		position: Position,
	) -> Result<Option<Vec<Entry>>> {
		// Create the language service request.
		let request = service::Request::Completion(service::completion::Request {
			module_identifier,
			position,
		});

		// Handle the language service request.
		let response = self.handle_language_service_request(request).await?;

		// Get the response.
		let service::Response::Completion(response) = response else { return_error!("Unexpected response type.") };

		Ok(response.entries)
	}
}
