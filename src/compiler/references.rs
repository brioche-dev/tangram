use super::{
	request::{ReferencesRequest, ReferencesResponse, Request, Response},
	Compiler, Location, ModuleIdentifier, Position,
};
use anyhow::{bail, Result};

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

		// Get the result from the response.
		let ReferencesResponse { locations } = response;

		Ok(locations)
	}
}
