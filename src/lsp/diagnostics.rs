use super::{send_notification, Sender};
use crate::{error::Result, language::Diagnostic, module, Instance};
use lsp_types as lsp;
use std::{collections::BTreeMap, sync::Arc};

impl Instance {
	pub async fn lsp_update_diagnostics(self: &Arc<Self>, sender: &Sender) -> Result<()> {
		// Perform the check.
		let diagnostics = self.diagnostics().await?;

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
