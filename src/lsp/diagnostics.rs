use super::{send_notification, Sender};
use crate::Cli;
use anyhow::Result;
use lsp_types as lsp;
use std::sync::Arc;

impl Cli {
	pub async fn lsp_update_diagnostics(self: &Arc<Self>, sender: &Sender) -> Result<()> {
		// Perform the check.
		let diagnostics = self.diagnostics().await?;

		// Publish the diagnostics.
		for (module_identifier, diagnostics) in diagnostics {
			let version = self
				.get_document_or_module_version(&module_identifier)
				.await
				.ok();
			let diagnostics = diagnostics.into_iter().map(Into::into).collect();
			send_notification::<lsp::notification::PublishDiagnostics>(
				sender,
				lsp::PublishDiagnosticsParams {
					uri: module_identifier.to_lsp_uri(),
					diagnostics,
					version,
				},
			);
		}

		Ok(())
	}
}
