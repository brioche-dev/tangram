use super::LanguageServer;
use crate::js;
use anyhow::Result;
use lsp_types as lsp;
use std::path::PathBuf;

impl LanguageServer {
	pub async fn references(
		&self,
		params: lsp::ReferenceParams,
	) -> Result<Option<Vec<lsp::Location>>> {
		// Get the position for the request.
		let position = params.text_document_position.position;

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
		let locations = self.compiler.get_references(url, position.into()).await?;
		let Some(locations) = locations else {
			return Ok(None);
		};

		// Convert the reference.
		let locations = locations
			.into_iter()
			.map(|location| {
				// Map the URL.
				let url = match location.url {
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

				lsp::Location {
					uri: url,
					range: location.range.into(),
				}
			})
			.collect();

		Ok(Some(locations))
	}
}
