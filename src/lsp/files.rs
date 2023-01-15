use super::{diagnostics::update_diagnostics, Sender};
use crate::Cli;
use anyhow::Result;
use lsp_types as lsp;
use std::path::Path;

pub async fn did_open(
	cli: Cli,
	sender: Sender,
	params: lsp::DidOpenTextDocumentParams,
) -> Result<()> {
	// Only proceed if the url has a file scheme.
	let scheme = params.text_document.uri.scheme();
	if scheme != "file" {
		return Ok(());
	}

	// Get the file path, version, and text.
	let path = Path::new(params.text_document.uri.path());
	let version = params.text_document.version;
	let text = params.text_document.text;

	// Open the file with the compiler.
	cli.open_file(path, version, text).await;

	// Update all diagnostics.
	update_diagnostics(&cli, &sender).await?;

	Ok(())
}

pub async fn did_change(
	cli: Cli,
	sender: Sender,
	params: lsp::DidChangeTextDocumentParams,
) -> Result<()> {
	// Get the file's path.
	let path = Path::new(params.text_document.uri.path());

	// Apply the changes.
	for change in params.content_changes {
		cli.change_file(
			path,
			params.text_document.version,
			change.range.map(Into::into),
			change.text,
		)
		.await;
	}

	// Update all diagnostics.
	update_diagnostics(&cli, &sender).await?;

	Ok(())
}

pub async fn did_close(
	cli: Cli,
	sender: Sender,
	params: lsp::DidCloseTextDocumentParams,
) -> Result<()> {
	// Get the document's path.
	let path = Path::new(params.text_document.uri.path());

	// Close the file in the compiler.
	cli.close_file(path).await;

	// Update all diagnostics.
	update_diagnostics(&cli, &sender).await?;

	Ok(())
}
