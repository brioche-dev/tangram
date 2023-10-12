use super::Server;
use crate::{
	convert_lsp_position, convert_range,
	module::{Location, Position},
	return_error, Module, Result,
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
	pub async fn handle_definition_request(
		&self,
		params: lsp::GotoDefinitionParams,
	) -> Result<Option<lsp::GotoDefinitionResponse>> {
		// Get the module.
		let module = self
			.convert_lsp_url(&params.text_document_position_params.text_document.uri)
			.await?;

		// Get the position for the request.
		let position = params.text_document_position_params.position;

		// Get the definitions.
		let locations = self
			.definition(&module, convert_lsp_position(position))
			.await?;

		let Some(locations) = locations else {
			return Ok(None);
		};

		// Convert the definitions.
		let locations = locations
			.into_iter()
			.map(|location| lsp::Location {
				uri: self.convert_module(&location.module),
				range: convert_range(location.range),
			})
			.collect();

		let response = lsp::GotoDefinitionResponse::Array(locations);

		Ok(Some(response))
	}

	pub async fn definition(
		&self,
		module: &Module,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the request.
		let request = super::Request::Definition(Request {
			module: module.clone(),
			position,
		});

		// Perform the request.
		let response = self.request(request).await?;

		// Get the response.
		let super::Response::Definition(response) = response else {
			return_error!("Unexpected response type.")
		};

		Ok(response.locations)
	}
}
