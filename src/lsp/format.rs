use crate::{language::Range, module, Cli};
use anyhow::Result;
use lsp_types as lsp;
use std::sync::Arc;

impl Cli {
	pub async fn lsp_format(
		self: &Arc<Self>,
		params: lsp::DocumentFormattingParams,
	) -> Result<Option<Vec<lsp::TextEdit>>> {
		// Get the module identifier.
		let module_identifier = module::Identifier::from_lsp_uri(params.text_document.uri).await?;

		// Load the module.
		let text = self.load_document_or_module(&module_identifier).await?;

		// Get the text range.
		let range = Range::from_byte_range_in_string(&text, 0..text.len());

		// Format the text.
		let formatted_text = self.format(text).await?;

		// Create the edit.
		let edit = lsp::TextEdit {
			range: range.into(),
			new_text: formatted_text,
		};

		Ok(Some(vec![edit]))
	}
}
