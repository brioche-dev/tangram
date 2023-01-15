use super::{CompletionEntry, ModuleIdentifier, Position, Request, Response};
use crate::Cli;
use anyhow::{bail, Result};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionRequest {
	pub module_identifier: ModuleIdentifier,
	pub position: Position,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionResponse {
	pub entries: Option<Vec<CompletionEntry>>,
}

impl Cli {
	pub async fn completion(
		&self,
		module_identifier: ModuleIdentifier,
		position: Position,
	) -> Result<Option<Vec<CompletionEntry>>> {
		// Create the request.
		let request = Request::Completion(CompletionRequest {
			module_identifier,
			position,
		});

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::Completion(response) => response,
			_ => bail!("Unexpected response type."),
		};

		Ok(response.entries)
	}
}
