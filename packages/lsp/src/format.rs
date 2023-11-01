use crate::{Range, Result, Server};
use lsp_types as lsp;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
	pub text: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	pub text: String,
}

impl Server {
	pub(super) async fn handle_format_request(
		&self,
		params: lsp::DocumentFormattingParams,
	) -> Result<Option<Vec<lsp::TextEdit>>> {
		let client = self.inner.client.upgrade().unwrap();

		// Get the module.
		let module = self.module_for_url(&params.text_document.uri).await?;

		// Load the module.
		let text = module
			.load(client.as_ref(), Some(&self.inner.document_store))
			.await?;

		// Get the text range.
		let range = Range::from_byte_range_in_string(&text, 0..text.len());

		// Format the text.
		let formatted_text = self.format(text).await?;

		// Create the edit.
		let edit = lsp::TextEdit {
			range: range.into(),
			new_text: formatted_text,
		};

		Ok(Some(vec![edit]))
	}

	pub async fn format(&self, text: String) -> Result<String> {
		// Create the request.
		let request = super::Request::Format(Request { text });

		// Perform the request.
		let response = self.request(request).await?.unwrap_format();

		Ok(response.text)
	}
}
