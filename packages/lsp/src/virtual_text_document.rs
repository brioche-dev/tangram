use crate::Server;
use lsp_types as lsp;
use tangram_client as tg;
use tg::Result;

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
	pub(super) async fn handle_virtual_text_document_request(
		&self,
		params: Params,
	) -> Result<Option<String>> {
		let client = self.inner.client.upgrade().unwrap();

		// Get the module.
		let module = self.module_for_url(&params.text_document.uri).await?;

		// Load the file.
		let text = module
			.load(client.as_ref(), Some(&self.inner.document_store))
			.await?;

		Ok(Some(text))
	}
}
