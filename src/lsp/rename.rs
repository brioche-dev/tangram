use super::{
	util::{from_uri, to_uri},
	LanguageServer,
};
use anyhow::Result;
use lsp_types as lsp;
use std::collections::HashMap;

impl LanguageServer {
	#[allow(clippy::similar_names)]
	pub async fn rename(&self, params: lsp::RenameParams) -> Result<Option<lsp::WorkspaceEdit>> {
		// Get the URL.
		let url = from_uri(params.text_document_position.text_document.uri).await?;

		// Get the position for the request.
		let position = params.text_document_position.position;
		let new_text = &params.new_name;

		// Get the references.
		let locations = self
			.compiler
			.find_rename_locations(url, position.into())
			.await?;

		let Some(locations) = locations else {
        return Ok(None);
      };

		// Convert the changes.
		let mut document_changes = HashMap::<url::Url, lsp::TextDocumentEdit>::new();
		for location in locations {
			// Get the version.
			let version = self.compiler.get_version(&location.url).await.ok();

			// Map the URL.
			let uri = to_uri(location.url);
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
			let changes_for_url = document_changes.get_mut(&uri).unwrap();
			changes_for_url.edits.push(lsp::OneOf::Left(lsp::TextEdit {
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
