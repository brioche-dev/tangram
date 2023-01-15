use super::util::{from_uri, to_uri};
use crate::Cli;
use anyhow::Result;
use lsp_types as lsp;

pub async fn references(
	cli: Cli,
	params: lsp::ReferenceParams,
) -> Result<Option<Vec<lsp::Location>>> {
	// Get the module identifier.
	let module_identifier = from_uri(params.text_document_position.text_document.uri).await?;

	// Get the position for the request.
	let position = params.text_document_position.position;

	// Get the references.
	let locations = cli.references(module_identifier, position.into()).await?;
	let Some(locations) = locations else {
			return Ok(None);
		};

	// Convert the reference.
	let locations = locations
		.into_iter()
		.map(|location| lsp::Location {
			uri: to_uri(location.module_identifier),
			range: location.range.into(),
		})
		.collect();

	Ok(Some(locations))
}
