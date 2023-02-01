use crate::Cli;
use anyhow::Result;
use lsp_types as lsp;

#[allow(clippy::unused_async)]
pub async fn initialize(
	_cli: Cli,
	_params: lsp::InitializeParams,
) -> Result<lsp::InitializeResult> {
	Ok(lsp::InitializeResult {
		capabilities: lsp::ServerCapabilities {
			hover_provider: Some(lsp::HoverProviderCapability::Simple(true)),
			references_provider: Some(lsp::OneOf::Left(true)),
			completion_provider: Some(lsp::CompletionOptions::default()),
			definition_provider: Some(lsp::OneOf::Left(true)),
			rename_provider: Some(lsp::OneOf::Left(true)),
			document_formatting_provider: Some(lsp::OneOf::Left(true)),
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
pub async fn shutdown(_cli: Cli, _params: ()) -> Result<()> {
	Ok(())
}
