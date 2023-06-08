use super::{send_notification, Sender, Server};
use crate::{error::Result, language::Diagnostic, module::Module};
use lsp_types as lsp;
use std::collections::BTreeMap;

impl Server {
	pub async fn update_diagnostics(&self, sender: &Sender) -> Result<()> {
		// Get the diagnostics.
		let diagnostics = Module::diagnostics(&self.tg).await?;

		// Clear the existing diagnostics.
		let mut existing_diagnostics = self.diagnostics.write().await;
		let mut diagnostics_for_module: BTreeMap<Module, Vec<Diagnostic>> = existing_diagnostics
			.drain(..)
			.filter_map(|diagnostic| {
				let module = diagnostic.location?.module;
				Some((module, Vec::new()))
			})
			.collect();

		// Add the new diagnostics.
		existing_diagnostics.extend(diagnostics.iter().cloned());
		for diagnostic in diagnostics {
			if let Some(location) = &diagnostic.location {
				diagnostics_for_module
					.entry(location.module.clone())
					.or_insert_with(Vec::new)
					.push(diagnostic);
			}
		}

		// Publish the diagnostics.
		for (module, diagnostics) in diagnostics_for_module {
			let version = Some(module.version(&self.tg).await?);
			let diagnostics = diagnostics.into_iter().map(Into::into).collect();
			send_notification::<lsp::notification::PublishDiagnostics>(
				sender,
				lsp::PublishDiagnosticsParams {
					uri: module.to_lsp(),
					diagnostics,
					version,
				},
			);
		}

		Ok(())
	}
}
