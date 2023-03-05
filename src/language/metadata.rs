use super::service;
use crate::{metadata::Metadata, Instance};
use anyhow::{bail, Result};
use std::sync::Arc;

impl Instance {
	#[allow(clippy::unused_async)]
	pub async fn metadata(self: &Arc<Self>, text: &str) -> Result<Metadata> {
		// Create the language service request.
		let request = service::Request::Metadata(service::metadata::Request {
			text: text.to_owned(),
		});

		// Send the language service request and receive the response.
		let response = self.language_service_request(request).await?;

		// Get the response.
		let service::Response::Metadata(response) = response else { bail!("Unexpected response type.") };

		// Get the text from the response.
		let service::metadata::Response { metadata } = response;

		Ok(metadata)
	}
}
