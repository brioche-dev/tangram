use crate::{error::Result, module, Instance};
use lsp_types as lsp;
use std::sync::Arc;

impl Instance {
	pub async fn lsp_definition(
		self: &Arc<Self>,
		params: lsp::GotoDefinitionParams,
	) -> Result<Option<lsp::GotoDefinitionResponse>> {
		// Get the module identifier.
		let module_identifier = module::Identifier::from_lsp_uri(
			params.text_document_position_params.text_document.uri,
		)
		.await?;

		// Get the position for the request.
		let position = params.text_document_position_params.position;

		// Get the definitions.
		let locations = self.definition(module_identifier, position.into()).await?;

		let Some(locations) = locations else {
			return Ok(None);
		};

		// Convert the definitions.
		let locations = locations
			.into_iter()
			.map(|location| lsp::Location {
				uri: location.module_identifier.to_lsp_uri(),
				range: location.range.into(),
			})
			.collect();

		let response = lsp::GotoDefinitionResponse::Array(locations);

		Ok(Some(response))
	}
}
