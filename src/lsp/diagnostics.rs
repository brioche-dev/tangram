use super::{send_notification, util::to_uri, Sender};
use crate::Cli;
use anyhow::Result;
use lsp_types as lsp;

pub async fn update_diagnostics(cli: &Cli, sender: &Sender) -> Result<()> {
	// Perform the check.
	let diagnostics = cli.diagnostics().await?;

	// Publish the diagnostics.
	for (url, diagnostics) in diagnostics {
		let version = cli.version(&url).await.ok();
		let diagnostics = diagnostics.into_iter().map(Into::into).collect();
		send_notification::<lsp::notification::PublishDiagnostics>(
			sender,
			lsp::PublishDiagnosticsParams {
				uri: to_uri(url),
				diagnostics,
				version,
			},
		);
	}

	Ok(())
}
