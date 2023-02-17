use super::Sender;
use crate::{module, Cli};
use anyhow::Result;
use lsp_types as lsp;
use std::sync::Arc;

impl Cli {
	pub async fn lsp_did_open(
		self: &Arc<Self>,
		sender: Sender,
		params: lsp::DidOpenTextDocumentParams,
	) -> Result<()> {
		// Get the module identifier.
		let module_identifier = module::Identifier::from_lsp_uri(params.text_document.uri).await?;

		// Open a document.
		let version = params.text_document.version;
		let text = params.text_document.text;
		self.open_document(&module_identifier, version, text).await;

		// Update all diagnostics.
		self.lsp_update_diagnostics(&sender).await?;

		Ok(())
	}

	pub async fn lsp_did_change(
		self: &Arc<Self>,
		sender: Sender,
		params: lsp::DidChangeTextDocumentParams,
	) -> Result<()> {
		// Get the module identifier.
		let module_identifier = module::Identifier::from_lsp_uri(params.text_document.uri).await?;

		// Apply the changes.
		for change in params.content_changes {
			self.update_document(
				&module_identifier,
				params.text_document.version,
				change.range.map(Into::into),
				change.text,
			)
			.await?;
		}

		// Update all diagnostics.
		self.lsp_update_diagnostics(&sender).await?;

		Ok(())
	}

	pub async fn lsp_did_close(
		self: &Arc<Self>,
		sender: Sender,
		params: lsp::DidCloseTextDocumentParams,
	) -> Result<()> {
		// Get the module identifier.
		let module_identifier = module::Identifier::from_lsp_uri(params.text_document.uri).await?;

		// Close the document.
		self.close_document(&module_identifier).await;

		// Update all diagnostics.
		self.lsp_update_diagnostics(&sender).await?;

		Ok(())
	}
}
