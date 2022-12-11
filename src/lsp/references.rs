use super::{
	util::{from_uri, to_uri},
	LanguageServer,
};
use anyhow::Result;
use lsp_types as lsp;

impl LanguageServer {
	pub async fn references(
		&self,
		params: lsp::ReferenceParams,
	) -> Result<Option<Vec<lsp::Location>>> {
		// Get the URL.
		let url = from_uri(params.text_document_position.text_document.uri).await?;

		// Get the position for the request.
		let position = params.text_document_position.position;

		// Get the references.
		let locations = self.compiler.get_references(url, position.into()).await?;
		let Some(locations) = locations else {
			return Ok(None);
		};

		// Convert the reference.
		let locations = locations
			.into_iter()
			.map(|location| lsp::Location {
				uri: to_uri(location.url),
				range: location.range.into(),
			})
			.collect();

		Ok(Some(locations))
	}
}
