use super::{util::to_uri, LanguageServer};
use crate::js;
use anyhow::Result;
use lsp_types as lsp;
use std::path::PathBuf;

impl LanguageServer {
	pub async fn definition(
		&self,
		params: lsp::GotoDefinitionParams,
	) -> Result<Option<lsp::GotoDefinitionResponse>> {
		// Get the position for the request.
		let position = params.text_document_position_params.position;

		// Parse the path.
		let path: PathBuf = params
			.text_document_position_params
			.text_document
			.uri
			.path()
			.parse()?;

		// Get the url for this path.
		let url = js::Url::for_path(&path).await?;

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
