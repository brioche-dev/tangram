use super::Server;
use crate::error::Result;
use lsp_types as lsp;

impl Server {
	pub fn initialize(&self, _params: &lsp::InitializeParams) -> Result<lsp::InitializeResult> {
		Ok(lsp::InitializeResult {
			capabilities: lsp::ServerCapabilities {
				text_document_sync: Some(lsp::TextDocumentSyncCapability::Options(
					lsp::TextDocumentSyncOptions {
						open_close: Some(true),
						change: Some(lsp::TextDocumentSyncKind::INCREMENTAL),
						..Default::default()
					},
				)),
				hover_provider: Some(lsp::HoverProviderCapability::Simple(true)),
				completion_provider: Some(lsp::CompletionOptions::default()),
				definition_provider: Some(lsp::OneOf::Left(true)),
				references_provider: Some(lsp::OneOf::Left(true)),
				document_formatting_provider: Some(lsp::OneOf::Left(true)),
				document_symbol_provider: Some(lsp::OneOf::Left(true)),
				rename_provider: Some(lsp::OneOf::Left(true)),

				..Default::default()
			},
			..Default::default()
		})
	}

	pub fn shutdown(&self, _params: ()) -> Result<()> {
		Ok(())
	}
}
