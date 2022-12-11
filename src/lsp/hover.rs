use super::{util::from_uri, LanguageServer};
use anyhow::Result;
use lsp_types as lsp;

impl LanguageServer {
	pub async fn hover(&self, params: lsp::HoverParams) -> Result<Option<lsp::Hover>> {
		// Get the URL.
		let url = from_uri(params.text_document_position_params.text_document.uri).await?;

		// Get the position for the request.
		let position = params.text_document_position_params.position;

		// Get the hover info.
		let hover = self.compiler.hover(url, position.into()).await?;
		let Some(hover) = hover else {
			return Ok(None);
		};

		// Create the hover.
		let hover = lsp::Hover {
			contents: lsp::HoverContents::Scalar(lsp::MarkedString::from_language_code(
				"typescript".into(),
				hover,
			)),
			range: None,
		};

		Ok(Some(hover))
	}
}
