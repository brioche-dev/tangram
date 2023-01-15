use super::{Request, Response};
use crate::Cli;
use anyhow::{bail, Result};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatRequest {
	pub text: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatResponse {
	pub text: String,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn format(&self, text: String) -> Result<String> {
		// Create the request.
		let request = Request::Format(FormatRequest { text });

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::Format(response) => response,
			_ => bail!("Unexpected response type."),
		};

		Ok(response.text)
	}
}
