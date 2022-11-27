use super::LanguageServer;
use crate::js;
use anyhow::Result;
use lsp_types as lsp;
use std::{collections::HashMap, path::PathBuf};

impl LanguageServer {
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
			let url: url::Url = match location.url {
				js::Url::PathModule {
					package_path,
					module_path,
				} => {
					let path = package_path.join(module_path);
					format!("file://{}", path.display()).parse().unwrap()
				},
				js::Url::Lib { .. }
				| js::Url::PackageModule { .. }
				| js::Url::PackageTargets { .. }
				| js::Url::PathTargets { .. } => location.url.into(),
			};
			if document_changes.get_mut(&url).is_none() {
				document_changes.insert(
					url.clone(),
					lsp::TextDocumentEdit {
						text_document: lsp::OptionalVersionedTextDocumentIdentifier {
							uri: url.clone(),
							version,
						},
						edits: Vec::<lsp::OneOf<lsp::TextEdit, lsp::AnnotatedTextEdit>>::new(),
					},
				);
			}
			let changes_for_url = document_changes.get_mut(&url).unwrap();
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
