use super::LanguageServer;
use crate::js;
use anyhow::Result;
use lsp_types as lsp;

impl LanguageServer {
	pub async fn update_diagnostics(&self) -> Result<()> {
		// Perform the check.
		let diagnostics = self.compiler.get_diagnostics().await?;

		// Publish the diagnostics.
		for (url, diagnostics) in diagnostics {
			let version = self.compiler.get_version(&url).await.ok();
			let path = match url {
				js::Url::PathModule {
					package_path,
					module_path,
				} => package_path.join(module_path),
				_ => continue,
			};
			let url = format!("file://{}", path.display()).parse().unwrap();
			let diagnostics = diagnostics.into_iter().map(Into::into).collect();
			self.send_notification::<lsp::notification::PublishDiagnostics>(
				lsp::PublishDiagnosticsParams {
					uri: url,
					diagnostics,
					version,
				},
			);
		}

		Ok(())
	}
}
