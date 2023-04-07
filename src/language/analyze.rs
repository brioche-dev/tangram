use super::service;
use crate::{
	error::{return_error, Result},
	instance::Instance,
	module::{self, Module},
	path::Path,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Output {
	pub imports: Vec<module::Specifier>,
	pub includes: Vec<Path>,
}

impl Module {
	#[allow(clippy::unused_async)]
	pub async fn analyze(tg: &Arc<Instance>, text: String) -> Result<Output> {
		// Create the language service request.
		let request = service::Request::Analyze(service::analyze::Request { text });

		// Handle the language service request.
		let response = tg.handle_language_service_request(request).await?;

		// Get the response.
		let service::Response::Analyze(response) = response else { return_error!("Unexpected response type.") };

		// Get the text from the response.
		let service::analyze::Response { includes, imports } = response;

		Ok(Output { imports, includes })
	}
}
