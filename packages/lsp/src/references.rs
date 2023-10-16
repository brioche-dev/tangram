use super::Server;
use crate::{convert_lsp_position, convert_range};
use lsp_types as lsp;
use tangram_client as tg;
use tg::{
	module::{Location, Module, Position},
	return_error, Result,
};

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
			.convert_lsp_url(&params.text_document_position.text_document.uri)
			.await?;

		// Get the position for the request.
		let position = params.text_document_position.position;

		// Get the references.
		let locations = self
			.references(&module, convert_lsp_position(position))
			.await?;
		let Some(locations) = locations else {
			return Ok(None);
		};

		// Convert the reference.
		let locations = locations
			.into_iter()
			.map(|location| lsp::Location {
				uri: self.convert_module(&location.module),
				range: convert_range(location.range),
			})
			.collect();

		Ok(Some(locations))
	}

	pub async fn references(
		&self,
		module: &Module,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the request.
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
