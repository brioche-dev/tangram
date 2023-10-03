use super::Server;
use crate::{
	language::{Module, Range},
	Result,
};
use lsp_types as lsp;

impl Server {
	pub async fn format(
		&self,
		params: lsp::DocumentFormattingParams,
	) -> Result<Option<Vec<lsp::TextEdit>>> {
		// Get the module.
		let module = Module::from_lsp(self, params.text_document.uri).await?;

		// Load the module.
		let text = module
			.load(&self.state.client, Some(&self.state.document_store))
			.await?;

		// Get the text range.
		let range = Range::from_byte_range_in_string(&text, 0..text.len());

		// Format the text.
		let formatted_text = Module::format(&self.state.language_service, text).await?;

		// Create the edit.
		let edit = lsp::TextEdit {
			range: range.into(),
			new_text: formatted_text,
		};

		Ok(Some(vec![edit]))
	}
}
