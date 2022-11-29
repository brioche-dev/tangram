use super::{util::to_uri, LanguageServer};
use crate::js;
use anyhow::Result;
use lsp_types as lsp;
use std::{collections::HashMap, path::PathBuf};

impl LanguageServer {
	#[allow(clippy::similar_names)]
	pub async fn rename(&self, params: lsp::RenameParams) -> Result<Option<lsp::WorkspaceEdit>> {
		// Get the position for the request.
		let position = params.text_document_position.position;
		let new_text = &params.new_name;

		// Parse the path.
		let path: PathBuf = params
			.text_document_position
			.text_document
			.uri
			.path()
			.parse()?;

		// Get the url for this path.
		let url = js::Url::for_path(&path).await?;

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
