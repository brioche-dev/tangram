use super::{send_notification, Sender, Server};
use crate::{error::Result, language::Diagnostic, module};
use lsp_types as lsp;
use std::collections::BTreeMap;

impl Server {
	pub async fn update_diagnostics(&self, sender: &Sender) -> Result<()> {
		// Perform the check.
		let diagnostics = self.tg.diagnostics().await?;

		// Collect the diagnostics by module identifier.
		let mut diagnostics_map: BTreeMap<module::Identifier, Vec<Diagnostic>> = BTreeMap::new();
		for diagnostic in diagnostics {
			if let Some(location) = &diagnostic.location {
				diagnostics_map
					.entry(location.module_identifier.clone())
					.or_insert_with(Vec::new)
					.push(diagnostic);
			}
		}

		// Publish the diagnostics.
		for (module_identifier, diagnostics) in diagnostics_map {
			let version = self
				.tg
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
