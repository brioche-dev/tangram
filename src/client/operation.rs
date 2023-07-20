use super::Client;
use crate::{error::Result, instance::Instance, operation::Operation, value::Value};

impl Client {
	pub async fn try_get_output(
		&self,
		tg: &Instance,
		operation: &Operation,
	) -> Result<Option<Value>> {
		// Build the URL.
		let id = operation.block().id();
		let path = format!("/v1/outputs/{id}");
		let mut url = self.url.clone();
		url.set_path(&path);

		// Send the request.
		let response = self.request(http::Method::GET, url).send().await?;

		// Check if the output exists.
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}

		// Check if the request was successful.
		let response = response.error_for_status()?;

		// Get the response body.
		let body = response.bytes().await?.to_vec();

		// Deserialize the output.
		let value = Value::from_bytes(tg, &body).await?;

		Ok(Some(value))
	}
}
