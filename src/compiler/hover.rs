use super::{ModuleIdentifier, Position, Request, Response};
use crate::Cli;
use anyhow::{bail, Result};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HoverRequest {
	pub module_identifier: ModuleIdentifier,
	pub position: Position,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoverResponse {
	pub text: Option<String>,
}

impl Cli {
	pub async fn hover(
		&self,
		module_identifier: ModuleIdentifier,
		position: Position,
	) -> Result<Option<String>> {
		// Create the request.
		let request = Request::Hover(HoverRequest {
			module_identifier,
			position,
		});

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::Hover(response) => response,
			_ => bail!("Unexpected response type."),
		};

		// Get the text from the response.
		let HoverResponse { text } = response;

		Ok(text)
	}
}
