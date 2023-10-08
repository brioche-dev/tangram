use super::{Sender, Server};
use crate::{Module, Result};
use lsp_types as lsp;

impl Server {
	pub(super) async fn handle_did_open_notification(
		&self,
		sender: Sender,
		params: lsp::DidOpenTextDocumentParams,
	) -> Result<()> {
		// Get the module.
		let module = self.module_for_url(&params.text_document.uri).await?;

		// Open the document.
		if let Module::Document(document) = module {
			let version = params.text_document.version;
			let text = params.text_document.text;
			document
				.open(&self.state.document_store, version, text)
				.await?;
		}

		// Update all diagnostics.
		self.update_diagnostics(&sender).await?;

		Ok(())
	}

	pub(super) async fn handle_did_change_notification(
		&self,
		sender: Sender,
		params: lsp::DidChangeTextDocumentParams,
	) -> Result<()> {
		// Get the module.
		let module = self.module_for_url(&params.text_document.uri).await?;

		if let Module::Document(document) = module {
			// Apply the changes.
			for change in params.content_changes {
				document
					.update(
						&self.state.document_store,
						change.range.map(Into::into),
						params.text_document.version,
						change.text,
					)
					.await?;
			}
		}

		// Update all diagnostics.
		self.update_diagnostics(&sender).await?;

		Ok(())
	}

	pub(super) async fn handle_did_close_notification(
		&self,
		sender: Sender,
		params: lsp::DidCloseTextDocumentParams,
	) -> Result<()> {
		// Get the module.
		let module = self.module_for_url(&params.text_document.uri).await?;

		if let Module::Document(document) = module {
			// Close the document.
			document.close(&self.state.document_store).await?;
		}

		// Update all diagnostics.
		self.update_diagnostics(&sender).await?;

		Ok(())
	}
}
