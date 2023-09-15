use super::Server;
use crate::{error::Result, module::Module};
use lsp_types as lsp;

impl Server {
	pub async fn hover(&self, params: lsp::HoverParams) -> Result<Option<lsp::Hover>> {
		// Get the module.
		let module = Module::from_lsp(
			&self.server,
			params.text_document_position_params.text_document.uri,
		)
		.await?;

		// Get the position for the request.
		let position = params.text_document_position_params.position;

		// Get the hover info.
		let hover = module.hover(&self.server, position.into()).await?;
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
