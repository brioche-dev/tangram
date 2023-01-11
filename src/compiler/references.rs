use super::{Compiler, Location, ModuleIdentifier, Position, Request, Response};
use anyhow::{bail, Result};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferencesRequest {
	pub module_identifier: ModuleIdentifier,
	pub position: Position,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferencesResponse {
	pub locations: Option<Vec<Location>>,
}

impl Compiler {
	pub async fn references(
		&self,
		module_identifier: ModuleIdentifier,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the request.
		let request = Request::References(ReferencesRequest {
			module_identifier,
			position,
		});

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::References(response) => response,
			_ => bail!("Unexpected response type."),
		};

		Ok(response.locations)
	}
}
