use super::Server;
use crate::{
	module::{Location, Module, Position},
	return_error, Result,
};
use lsp_types as lsp;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
	pub module: Module,
	pub position: Position,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	pub locations: Option<Vec<Location>>,
}

impl Server {
	pub(super) async fn handle_references_request(
		&self,
		params: lsp::ReferenceParams,
	) -> Result<Option<Vec<lsp::Location>>> {
		// Get the module.
		let module = self
			.module_for_url(&params.text_document_position.text_document.uri)
			.await?;

		// Get the position for the request.
		let position = params.text_document_position.position;

		// Get the references.
		let locations = self.references(&module, position.into()).await?;
		let Some(locations) = locations else {
			return Ok(None);
		};

		// Convert the reference.
		let locations = locations
			.into_iter()
			.map(|location| lsp::Location {
				uri: self.url_for_module(&location.module),
				range: location.range.into(),
			})
			.collect();

		Ok(Some(locations))
	}

	pub async fn references(
		&self,
		module: &Module,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the language service request.
		let request = super::Request::References(Request {
			module: module.clone(),
			position,
		});

		// Perform the request.
		let response = self.request(request).await?;

		// Get the response.
		let super::Response::References(response) = response else {
			return_error!("Unexpected response type.");
		};

		Ok(response.locations)
	}
}
