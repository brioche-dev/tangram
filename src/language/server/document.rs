use super::{Sender, Server};
use crate::{error::Result, module::Module};
use lsp_types as lsp;

impl Server {
	pub async fn did_open(
		&self,
		sender: Sender,
		params: lsp::DidOpenTextDocumentParams,
	) -> Result<()> {
		// Get the module.
		let module = Module::from_lsp(&self.server, params.text_document.uri).await?;

		// Open the document.
		if let Module::Document(document) = module {
			let version = params.text_document.version;
			let text = params.text_document.text;
			document.open(&self.server, version, text).await?;
		}

		// Update all diagnostics.
		self.update_diagnostics(&sender).await?;

		Ok(())
	}

	pub async fn did_change(
		&self,
		sender: Sender,
		params: lsp::DidChangeTextDocumentParams,
	) -> Result<()> {
		// Get the module.
		let module = Module::from_lsp(&self.server, params.text_document.uri).await?;

		if let Module::Document(document) = module {
			// Apply the changes.
			for change in params.content_changes {
				document
					.update(
						&self.server,
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

	pub async fn did_close(
		&self,
		sender: Sender,
		params: lsp::DidCloseTextDocumentParams,
	) -> Result<()> {
		// Get the module.
		let module = Module::from_lsp(&self.server, params.text_document.uri).await?;

		if let Module::Document(document) = module {
			// Close the document.
			document.close(&self.server).await?;
		}

		// Update all diagnostics.
		self.update_diagnostics(&sender).await?;

		Ok(())
	}
}
