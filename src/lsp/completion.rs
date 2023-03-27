use super::Server;
use crate::{error::Result, module};
use lsp_types as lsp;

impl Server {
	pub async fn completion(
		&self,
		params: lsp::CompletionParams,
	) -> Result<Option<lsp::CompletionResponse>> {
		// Get the module identifier.
		let module_identifier =
			module::Identifier::from_lsp_uri(params.text_document_position.text_document.uri)
				.await?;

		// Get the position for the request.
		let position = params.text_document_position.position;

		// Get the completion entries.
		let entries = self
			.tg
			.completion(module_identifier, position.into())
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
