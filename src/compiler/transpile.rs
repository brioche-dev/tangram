use super::{
	request::{Request, Response, TranspileRequest},
	Compiler, TranspileOutput,
};
use anyhow::{bail, Result};

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
