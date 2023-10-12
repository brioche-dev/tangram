use super::Server;
use lsp_types as lsp;

impl Server {
	pub(super) fn handle_initialize_request(
		_params: &lsp::InitializeParams,
	) -> lsp::InitializeResult {
		lsp::InitializeResult {
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
		}
	}
}
