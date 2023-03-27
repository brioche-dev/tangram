use super::Server;
use crate::{error::Result, language::Range, module};
use lsp_types as lsp;

impl Server {
	pub async fn format(
		&self,
		params: lsp::DocumentFormattingParams,
	) -> Result<Option<Vec<lsp::TextEdit>>> {
		// Get the module identifier.
		let module_identifier = module::Identifier::from_lsp_uri(params.text_document.uri).await?;

		// Load the module.
		let text = self.tg.load_document_or_module(&module_identifier).await?;

		// Get the text range.
		let range = Range::from_byte_range_in_string(&text, 0..text.len());

		// Format the text.
		let formatted_text = self.tg.format(text).await?;

		// Create the edit.
		let edit = lsp::TextEdit {
			range: range.into(),
			new_text: formatted_text,
		};

		Ok(Some(vec![edit]))
	}
}
