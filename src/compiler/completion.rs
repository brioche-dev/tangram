use super::{
	request::{CompletionRequest, Request, Response},
	Compiler, CompletionEntry, ModuleIdentifier, Position,
};
use anyhow::{bail, Result};

impl Compiler {
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

		// Get the result from the response.
		let entries = response.entries;

		Ok(entries)
	}
}
