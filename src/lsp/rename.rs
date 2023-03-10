use crate::{error::Result, module, Instance};
use lsp_types as lsp;
use std::{collections::HashMap, sync::Arc};
use url::Url;

impl Instance {
	#[allow(clippy::similar_names)]
	pub async fn lsp_rename(
		self: &Arc<Self>,
		params: lsp::RenameParams,
	) -> Result<Option<lsp::WorkspaceEdit>> {
		// Get the module identifier.
		let module_identifier =
			module::Identifier::from_lsp_uri(params.text_document_position.text_document.uri)
				.await?;

		// Get the position for the request.
		let position = params.text_document_position.position;
		let new_text = &params.new_name;

		// Get the references.
		let locations = self.rename(module_identifier, position.into()).await?;

		// If there are no references, then return None.
		let Some(locations) = locations else {
		return Ok(None);
	};

		// Convert the changes.
		let mut document_changes = HashMap::<Url, lsp::TextDocumentEdit>::new();
		for location in locations {
			// Get the version.
			let version = self
				.get_document_or_module_version(&location.module_identifier)
				.await
				.ok();

			// Create the URI.
			let uri = location.module_identifier.to_lsp_uri();

			if document_changes.get_mut(&uri).is_none() {
				document_changes.insert(
					uri.clone(),
					lsp::TextDocumentEdit {
						text_document: lsp::OptionalVersionedTextDocumentIdentifier {
							uri: uri.clone(),
							version,
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
