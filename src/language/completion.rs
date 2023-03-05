use super::{service, Position};
use crate::{module, Instance};
use anyhow::{bail, Result};
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

		// Send the language service request and receive the response.
		let response = self.language_service_request(request).await?;

		// Get the response.
		let service::Response::Completion(response) = response else { bail!("Unexpected response type.") };

		Ok(response.entries)
	}
}
