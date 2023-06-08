use super::Server;
use crate::{error::Result, module::Module};
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
		// Get the module.
		let module = Module::from_lsp(&self.tg, params.text_document.uri).await?;

		// Load the file.
		let text = module.load(&self.tg).await?;

		Ok(Some(text))
	}
}
