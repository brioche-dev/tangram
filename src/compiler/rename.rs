use super::{Location, ModuleIdentifier, Position, Request, Response};
use crate::Cli;
use anyhow::{bail, Result};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameRequest {
	pub module_identifier: ModuleIdentifier,
	pub position: Position,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameResponse {
	pub locations: Option<Vec<Location>>,
}

impl Cli {
	pub async fn rename(
		&self,
		module_identifier: ModuleIdentifier,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the request.
		let request = Request::Rename(RenameRequest {
			module_identifier,
			position,
		});

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::Rename(response) => response,
			_ => bail!("Unexpected response type."),
		};

		Ok(response.locations)
	}
}
