use super::{
	request::{HoverRequest, HoverResponse, Request, Response},
	Compiler, ModuleIdentifier, Position,
};
use anyhow::{bail, Result};

impl Compiler {
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
