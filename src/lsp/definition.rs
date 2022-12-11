use super::{
	util::{from_uri, to_uri},
	LanguageServer,
};
use anyhow::Result;
use lsp_types as lsp;

impl LanguageServer {
	pub async fn definition(
		&self,
		params: lsp::GotoDefinitionParams,
	) -> Result<Option<lsp::GotoDefinitionResponse>> {
		// Get the URL.
		let url = from_uri(params.text_document_position_params.text_document.uri).await?;

		// Get the position for the request.
		let position = params.text_document_position_params.position;

		// Get the definitions.
		let locations = self.compiler.goto_definition(url, position.into()).await?;

		let Some(locations) = locations else {
			return Ok(None);
		};

		// Convert the definitions.
		let locations = locations
			.into_iter()
			.map(|location| lsp::Location {
				uri: to_uri(location.url),
				range: location.range.into(),
			})
			.collect();

		let response = lsp::GotoDefinitionResponse::Array(locations);

		Ok(Some(response))
	}
}
