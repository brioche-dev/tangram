use super::Server;
use crate::{language::Module, Result};
use lsp_types as lsp;

impl Server {
	pub async fn completion(
		&self,
		params: lsp::CompletionParams,
	) -> Result<Option<lsp::CompletionResponse>> {
		// Get the module.
		let module =
			Module::from_lsp(self, params.text_document_position.text_document.uri).await?;

		// Get the position for the request.
		let position = params.text_document_position.position;

		// Get the completion entries.
		let entries = module
			.completion(&self.state.language_service, position.into())
			.await?;
		let Some(entries) = entries else {
			return Ok(None);
		};

		// Convert the completion entries.
		let entries = entries
			.into_iter()
			.map(|completion| lsp::CompletionItem {
				label: completion.name,
				..Default::default()
			})
			.collect();

		Ok(Some(lsp::CompletionResponse::Array(entries)))
	}
}
