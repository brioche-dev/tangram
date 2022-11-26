use super::LanguageServer;
use anyhow::Result;
use lsp_types as lsp;
use std::path::Path;

impl LanguageServer {
	pub async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) -> Result<()> {
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
		self.compiler.open_file(path, version, text).await;

		// Update all diagnostics.
		self.update_diagnostics().await?;

		Ok(())
	}

	pub async fn did_change(&self, params: lsp::DidChangeTextDocumentParams) -> Result<()> {
		// Get the file's path.
		let path = Path::new(params.text_document.uri.path());

		// Update the document in the compiler.
		for change in params.content_changes {
			self.compiler
				.change_file(path, params.text_document.version, change.text)
				.await;
		}

		// Update all diagnostics.
		self.update_diagnostics().await?;

		Ok(())
	}

	pub async fn did_close(&self, params: lsp::DidCloseTextDocumentParams) -> Result<()> {
		// Get the document's path.
		let path = Path::new(params.text_document.uri.path());

		// Close the file in the compiler.
		self.compiler.close_file(path).await;

		// Update all diagnostics.
		self.update_diagnostics().await?;

		Ok(())
	}
}
