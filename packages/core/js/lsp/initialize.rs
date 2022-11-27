use super::LanguageServer;
use anyhow::Result;
use lsp_types as lsp;

impl LanguageServer {
	#[allow(clippy::unused_async)]
	pub async fn initialize(
		&self,
		_params: lsp::InitializeParams,
	) -> Result<lsp::InitializeResult> {
		Ok(lsp::InitializeResult {
			capabilities: lsp::ServerCapabilities {
				hover_provider: Some(lsp::HoverProviderCapability::Simple(true)),
				references_provider: Some(lsp::OneOf::Left(true)),
				completion_provider: Some(lsp::CompletionOptions::default()),
				definition_provider: Some(lsp::OneOf::Left(true)),
				rename_provider: Some(lsp::OneOf::Left(true)),
				text_document_sync: Some(lsp::TextDocumentSyncCapability::Options(
					lsp::TextDocumentSyncOptions {
						open_close: Some(true),
						change: Some(lsp::TextDocumentSyncKind::INCREMENTAL),
						..Default::default()
					},
				)),
				..Default::default()
			},
			..Default::default()
		})
	}

	#[allow(clippy::unused_async)]
	pub async fn shutdown(&self, _params: ()) -> Result<()> {
		Ok(())
	}
}
