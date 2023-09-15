use super::Server;
use crate::{error::Result, module::Module};
use lsp_types as lsp;
use std::collections::HashMap;
use url::Url;

impl Server {
	#[allow(clippy::similar_names)]
	pub async fn rename(&self, params: lsp::RenameParams) -> Result<Option<lsp::WorkspaceEdit>> {
		// Get the module.
		let module = Module::from_lsp(
			&self.server,
			params.text_document_position.text_document.uri,
		)
		.await?;

		// Get the position for the request.
		let position = params.text_document_position.position;
		let new_text = &params.new_name;

		// Get the references.
		let locations = module.rename(&self.server, position.into()).await?;

		// If there are no references, then return None.
		let Some(locations) = locations else {
		return Ok(None);
	};

		// Convert the changes.
		let mut document_changes = HashMap::<Url, lsp::TextDocumentEdit>::new();
		for location in locations {
			// Get the version.
			let version = location.module.version(&self.server).await?;

			// Create the URI.
			let uri = location.module.to_lsp();

			if document_changes.get_mut(&uri).is_none() {
				document_changes.insert(
					uri.clone(),
					lsp::TextDocumentEdit {
						text_document: lsp::OptionalVersionedTextDocumentIdentifier {
							uri: uri.clone(),
							version: Some(version),
						},
						edits: Vec::<lsp::OneOf<lsp::TextEdit, lsp::AnnotatedTextEdit>>::new(),
					},
				);
			}

			document_changes
				.get_mut(&uri)
				.unwrap()
				.edits
				.push(lsp::OneOf::Left(lsp::TextEdit {
					range: location.range.into(),
					new_text: new_text.clone(),
				}));
		}

		let changes = lsp::WorkspaceEdit {
			changes: None,
			document_changes: Some(lsp::DocumentChanges::Edits(
				document_changes.values().cloned().collect(),
			)),
			change_annotations: None,
		};

		Ok(Some(changes))
	}
}
