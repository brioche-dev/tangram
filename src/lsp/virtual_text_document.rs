use crate::Cli;
use anyhow::Result;
use lsp_types as lsp;

pub struct VirtualTextDocument;

impl lsp::request::Request for VirtualTextDocument {
	type Params = Params;
	type Result = Option<String>;
	const METHOD: &'static str = "tangram/virtualTextDocument";
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Params {
	pub text_document: lsp::TextDocumentIdentifier,
}

#[allow(clippy::unused_async)]
pub async fn virtual_text_document(cli: Cli, params: Params) -> Result<Option<String>> {
	// Get the module identifier.
	let module_identifier = params.text_document.uri.try_into()?;

	// Load the file.
	let text = cli.load(&module_identifier).await?;

	Ok(Some(text))
}
