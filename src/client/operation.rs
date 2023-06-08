use super::Client;
use crate::{
	error::{Result, WrapErr},
	operation, value,
};

impl Client {
	pub async fn try_get_operation(
		&self,
		operation_hash: operation::Hash,
	) -> Result<Option<operation::Data>> {
		// Create the path.
		let path = format!("/v1/operations/{operation_hash}");

		// Build the URL.
		let mut url = self.url.clone();
		url.set_path(&path);

		// Send the request.
		let response = self
			.request(http::Method::GET, url)
			.send()
			.await?
			.error_for_status()?;

		// Read the response body.
		let response = response
			.json()
			.await
			.wrap_err("Failed to read the response body.")?;

		Ok(response)
	}

	pub async fn try_get_operation_output(
		&self,
		operation_hash: operation::Hash,
	) -> Result<Option<value::Data>> {
		// Create the path.
		let path = format!("/v1/operations/{operation_hash}/output");

		// Build the URL.
		let mut url = self.url.clone();
		url.set_path(&path);

		// Send the request.
		let response = self
			.request(http::Method::GET, url)
			.send()
			.await?
			.error_for_status()?;

		// Read the response body.
		let response = response
			.json()
			.await
			.wrap_err("Failed to read the response body.")?;

		Ok(response)
	}
}
