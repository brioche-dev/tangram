use super::{send_notification, Sender, Server};
use crate::{module::Diagnostic, return_error, Module, Result};
use lsp_types as lsp;
use std::collections::BTreeMap;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	pub diagnostics: Vec<Diagnostic>,
}

impl Server {
	pub async fn update_diagnostics(&self, sender: &Sender) -> Result<()> {
		// Get the diagnostics.
		let diagnostics = self.diagnostics().await?;

		// Clear the existing diagnostics.
		let mut existing_diagnostics = self.state.diagnostics.write().await;
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
			let version = Some(module.version(Some(&self.state.document_store)).await?);
			let diagnostics = diagnostics.into_iter().map(Into::into).collect();
			send_notification::<lsp::notification::PublishDiagnostics>(
				sender,
				lsp::PublishDiagnosticsParams {
					uri: self.url_for_module(&module),
					diagnostics,
					version,
				},
			);
		}

		Ok(())
	}

	pub async fn diagnostics(&self) -> Result<Vec<Diagnostic>> {
		// Create the language service request.
		let request = super::Request::Diagnostics(Request {});

		// Perform the request.
		let response = self.request(request).await?;

		// Get the response.
		let super::Response::Diagnostics(response) = response else {
			return_error!("Unexpected response type.")
		};

		// Get the result the response.
		let Response { diagnostics } = response;

		Ok(diagnostics)
	}
}
