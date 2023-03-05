use super::service;
use crate::{module, Instance};
use anyhow::{bail, Result};
use std::sync::Arc;

// #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub type Doc = serde_json::Value;

impl Instance {
	/// Get the docs for a module.
	pub async fn doc(self: &Arc<Self>, module_identifier: module::Identifier) -> Result<Doc> {
		// Create the language service request.
		let request = service::Request::Doc(service::doc::Request { module_identifier });

		// Send the language service request and receive the response.
		let response = self.language_service_request(request).await?;

		// Get the response.
		let service::Response::Doc(response) = response else { bail!("Unexpected response type.") };

		Ok(response.doc)
	}
}
