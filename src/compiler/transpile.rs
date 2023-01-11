use super::{Compiler, Request, Response, TranspileOutput};
use anyhow::{bail, Result};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranspileRequest {
	pub text: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranspileResponse {
	pub output_text: String,
	pub source_map_text: String,
}

impl Compiler {
	pub async fn transpile(&self, text: String) -> Result<TranspileOutput> {
		// Create the request.
		let request = Request::Transpile(TranspileRequest { text });

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::Transpile(response) => response,
			_ => bail!("Unexpected response type."),
		};

		Ok(TranspileOutput {
			transpiled: response.output_text,
			source_map: response.source_map_text,
		})
	}
}
