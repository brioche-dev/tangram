use super::util::{from_uri, to_uri};
use crate::Cli;
use anyhow::Result;
use lsp_types as lsp;

pub async fn definition(
	cli: Cli,
	params: lsp::GotoDefinitionParams,
) -> Result<Option<lsp::GotoDefinitionResponse>> {
	// Get the module identifier.
	let module_identifier =
		from_uri(params.text_document_position_params.text_document.uri).await?;

	// Get the position for the request.
	let position = params.text_document_position_params.position;

	// Get the definitions.
	let locations = cli.definition(module_identifier, position.into()).await?;

	let Some(locations) = locations else {
			return Ok(None);
		};

	// Convert the definitions.
	let locations = locations
		.into_iter()
		.map(|location| lsp::Location {
			uri: to_uri(location.module_identifier),
			range: location.range.into(),
		})
		.collect();

	let response = lsp::GotoDefinitionResponse::Array(locations);

	Ok(Some(response))
}
