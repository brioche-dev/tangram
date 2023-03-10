use crate::{error::Result, module, Instance};
use lsp_types as lsp;
use std::sync::Arc;

impl Instance {
	pub async fn lsp_references(
		self: &Arc<Self>,
		params: lsp::ReferenceParams,
	) -> Result<Option<Vec<lsp::Location>>> {
		// Get the module identifier.
		let module_identifier =
			module::Identifier::from_lsp_uri(params.text_document_position.text_document.uri)
				.await?;

		// Get the position for the request.
		let position = params.text_document_position.position;

		// Get the references.
		let locations = self.references(module_identifier, position.into()).await?;
		let Some(locations) = locations else {
			return Ok(None);
		};

		// Convert the reference.
		let locations = locations
			.into_iter()
			.map(|location| lsp::Location {
				uri: location.module_identifier.to_lsp_uri(),
				range: location.range.into(),
			})
			.collect();

		Ok(Some(locations))
	}
}
