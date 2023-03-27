use super::Server;
use crate::error::Result;
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

impl Server {
	#[allow(clippy::unused_async)]
	pub async fn virtual_text_document(&self, params: Params) -> Result<Option<String>> {
		// Get the module identifier.
		let module_identifier = params.text_document.uri.try_into()?;

		// Load the file.
		let text = self.tg.load_document_or_module(&module_identifier).await?;

		Ok(Some(text))
	}
}
