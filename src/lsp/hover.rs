use crate::{module, Instance};
use anyhow::Result;
use lsp_types as lsp;
use std::sync::Arc;

impl Instance {
	pub async fn lsp_hover(
		self: &Arc<Self>,
		params: lsp::HoverParams,
	) -> Result<Option<lsp::Hover>> {
		// Get the module identifier.
		let module_identifier = module::Identifier::from_lsp_uri(
			params.text_document_position_params.text_document.uri,
		)
		.await?;

		// Get the position for the request.
		let position = params.text_document_position_params.position;

		// Get the hover info.
		let hover = self.hover(module_identifier, position.into()).await?;
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
