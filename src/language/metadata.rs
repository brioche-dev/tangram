use super::service;
use crate::{
	error::{return_error, Result},
	instance::Instance,
	module::Module,
	package,
};
use std::sync::Arc;

impl Module {
	#[allow(clippy::unused_async)]

	pub async fn metadata(tg: &Arc<Instance>, text: &str) -> Result<package::Metadata> {
		// Create the language service request.
		let request = service::Request::Metadata(service::metadata::Request {
			text: text.to_owned(),
		});

		// Handle the language service request.
		let response = tg.handle_language_service_request(request).await?;

		// Get the response.
		let service::Response::Metadata(response) = response else { return_error!("Unexpected response type.") };

		// Get the text from the response.
		let service::metadata::Response { metadata } = response;

		Ok(metadata)
	}
}
