use super::LanguageServer;
use crate::js;
use anyhow::Result;
use lsp_types as lsp;
use std::path::PathBuf;

impl LanguageServer {
	pub async fn hover(&self, params: lsp::HoverParams) -> Result<Option<lsp::Hover>> {
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
