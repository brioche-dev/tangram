use super::Server;
use crate::{error::Result, module::Module};
use lsp_types as lsp;

impl Server {
	pub async fn references(
		&self,
		params: lsp::ReferenceParams,
	) -> Result<Option<Vec<lsp::Location>>> {
		// Get the module.
		let module = Module::from_lsp(
			&self.server,
			params.text_document_position.text_document.uri,
		)
		.await?;

		// Get the position for the request.
		let position = params.text_document_position.position;

		// Get the references.
		let locations = module.references(&self.server, position.into()).await?;
		let Some(locations) = locations else {
			return Ok(None);
		};

		// Convert the reference.
		let locations = locations
			.into_iter()
			.map(|location| lsp::Location {
				uri: location.module.to_lsp(),
				range: location.range.into(),
			})
			.collect();

		Ok(Some(locations))
	}
}
