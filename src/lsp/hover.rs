use super::util::from_uri;
use crate::Cli;
use anyhow::Result;
use lsp_types as lsp;

pub async fn hover(cli: Cli, params: lsp::HoverParams) -> Result<Option<lsp::Hover>> {
	// Get the module identifier.
	let module_identifier =
		from_uri(params.text_document_position_params.text_document.uri).await?;

	// Get the position for the request.
	let position = params.text_document_position_params.position;

	// Get the hover info.
	let hover = cli.hover(module_identifier, position.into()).await?;
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
