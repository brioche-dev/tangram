use crate::Cli;

use super::util::from_uri;
use anyhow::Result;
use lsp_types as lsp;

pub async fn formatting(
	cli: Cli,
	params: lsp::DocumentFormattingParams,
) -> Result<Option<Vec<lsp::TextEdit>>> {
	// Get the module identifier.
	let module_identifier = from_uri(params.text_document.uri).await?;

	// Format the module, returning a list of edits.
	let edits = cli.format(module_identifier).await?;
	let Some(edits) = edits else {
		return Ok(None);
	};

	// Convert the formatting edits.
	let edits = edits
		.into_iter()
		.map(|edit| {
			anyhow::Ok(lsp::TextEdit {
				range: edit.range.into(),
				new_text: edit.new_text,
			})
		})
		.collect::<anyhow::Result<Vec<_>>>()?;

	Ok(Some(edits))
}
