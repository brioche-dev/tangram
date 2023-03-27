use super::Server;
use crate::{error::Result, module};
use lsp_types as lsp;

impl Server {
	pub async fn hover(&self, params: lsp::HoverParams) -> Result<Option<lsp::Hover>> {
		// Get the module identifier.
		let module_identifier = module::Identifier::from_lsp_uri(
			params.text_document_position_params.text_document.uri,
		)
		.await?;

		// Get the position for the request.
		let position = params.text_document_position_params.position;

		// Get the hover info.
		let hover = self.tg.hover(module_identifier, position.into()).await?;
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
