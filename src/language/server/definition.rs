use super::Server;
use crate::{language::Module, Result};
use lsp_types as lsp;

impl Server {
	pub async fn definition(
		&self,
		params: lsp::GotoDefinitionParams,
	) -> Result<Option<lsp::GotoDefinitionResponse>> {
		// Get the module.
		let module = Module::from_lsp(
			&self,
			params.text_document_position_params.text_document.uri,
		)
		.await?;

		// Get the position for the request.
		let position = params.text_document_position_params.position;

		// Get the definitions.
		let locations = module
			.definition(&self.state.language_service, position.into())
			.await?;

		let Some(locations) = locations else {
			return Ok(None);
		};

		// Convert the definitions.
		let locations = locations
			.into_iter()
			.map(|location| lsp::Location {
				uri: location.module.to_lsp(),
				range: location.range.into(),
			})
			.collect();

		let response = lsp::GotoDefinitionResponse::Array(locations);

		Ok(Some(response))
	}
}
