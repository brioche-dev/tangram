use super::LanguageServer;
use crate::js;
use anyhow::Result;
use lsp_types as lsp;
use std::path::PathBuf;

impl LanguageServer {
	pub async fn completion(
		&self,
		params: lsp::CompletionParams,
	) -> Result<Option<lsp::CompletionResponse>> {
		// Get the position for the request.
		let position = params.text_document_position.position;

		// Parse the path.
		let path: PathBuf = params
			.text_document_position
			.text_document
			.uri
			.path()
			.parse()?;

		// Get the url for this path.
		let url = js::Url::for_path(&path).await?;

		// Get the completion entries.
		let entries = self.compiler.completion(url, position.into()).await?;
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
