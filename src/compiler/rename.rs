use super::{
	request::{RenameLocationsResponse, RenameRequest, Request, Response},
	Compiler, Location, ModuleIdentifier, Position,
};
use anyhow::{bail, Result};

impl Compiler {
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

		// Get the result from the response.
		let RenameLocationsResponse { locations } = response;

		Ok(locations)
	}
}
