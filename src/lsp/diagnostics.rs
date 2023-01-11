use super::{util::to_uri, LanguageServer};
use anyhow::Result;
use lsp_types as lsp;

impl LanguageServer {
	pub async fn update_diagnostics(&self) -> Result<()> {
		// Perform the check.
		let diagnostics = self.compiler.diagnostics().await?;

		// Publish the diagnostics.
		for (url, diagnostics) in diagnostics {
			let version = self.compiler.version(&url).await.ok();
			let diagnostics = diagnostics.into_iter().map(Into::into).collect();
			self.send_notification::<lsp::notification::PublishDiagnostics>(
				lsp::PublishDiagnosticsParams {
					uri: to_uri(url),
					diagnostics,
					version,
				},
			);
		}

		Ok(())
	}
}
