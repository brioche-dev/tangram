use super::{Sender, Server};
use crate::{error::Result, module};
use lsp_types as lsp;

impl Server {
	pub async fn did_open(
		&self,
		sender: Sender,
		params: lsp::DidOpenTextDocumentParams,
	) -> Result<()> {
		// Get the module identifier.
		let module_identifier = module::Identifier::from_lsp_uri(params.text_document.uri).await?;

		// Open a document.
		let version = params.text_document.version;
		let text = params.text_document.text;
		self.tg
			.open_document(&module_identifier, version, text)
			.await;

		// Update all diagnostics.
		self.update_diagnostics(&sender).await?;

		Ok(())
	}

	pub async fn did_change(
		&self,
		sender: Sender,
		params: lsp::DidChangeTextDocumentParams,
	) -> Result<()> {
		// Get the module identifier.
		let module_identifier = module::Identifier::from_lsp_uri(params.text_document.uri).await?;

		// Apply the changes.
		for change in params.content_changes {
			self.tg
				.update_document(
					&module_identifier,
					params.text_document.version,
					change.range.map(Into::into),
					change.text,
				)
				.await?;
		}

		// Update all diagnostics.
		self.update_diagnostics(&sender).await?;

		Ok(())
	}

	pub async fn did_close(
		&self,
		sender: Sender,
		params: lsp::DidCloseTextDocumentParams,
	) -> Result<()> {
		// Get the module identifier.
		let module_identifier = module::Identifier::from_lsp_uri(params.text_document.uri).await?;

		// Close the document.
		self.tg.close_document(&module_identifier).await;

		// Update all diagnostics.
		self.update_diagnostics(&sender).await?;

		Ok(())
	}
}
