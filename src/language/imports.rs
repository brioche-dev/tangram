use super::service;
use crate::{
	error::{return_error, Result},
	module, Instance,
};
use std::sync::Arc;

impl Instance {
	#[allow(clippy::unused_async)]
	pub async fn imports(self: &Arc<Self>, text: &str) -> Result<Vec<module::Specifier>> {
		// Create the language service request.
		let request = service::Request::Imports(service::imports::Request {
			text: text.to_owned(),
		});

		// Send the language service request and receive the response.
		let response = self.language_service_request(request).await?;

		// Get the response.
		let service::Response::Imports(response) = response else { return_error!("Unexpected response type.") };

		// Get the text from the response.
		let service::imports::Response { imports } = response;

		Ok(imports)
	}
}
