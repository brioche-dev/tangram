use super::service;
use crate::{
	error::{return_error, Result},
	instance::Instance,
	module::Module,
};
use std::{collections::BTreeMap, sync::Arc};

// #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub type Symbol = serde_json::Value;

impl Module {
	/// Get the docs for a module.
	pub async fn doc(&self, tg: &Arc<Instance>) -> Result<BTreeMap<String, Symbol>> {
		// Create the language service request.
		let request = service::Request::Doc(service::doc::Request {
			module: self.clone(),
		});

		// Handle the language service request.
		let response = tg.handle_language_service_request(request).await?;

		// Get the response.
		let service::Response::Doc(response) = response else { return_error!("Unexpected response type.") };

		Ok(response.exports)
	}
}
