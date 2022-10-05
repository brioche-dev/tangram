use super::Client;
use crate::hash::Hash;
use anyhow::Result;

impl Client {
	pub async fn evaluate(&self, hash: Hash) -> Result<Hash> {
		// Build the URL.
		let path = format!("/expressions/{hash}/evaluate");
		let mut url = self.url.clone();
		url.set_path(&path);

		// Send the request.
		let response = self
			.create_request(http::Method::POST, url.to_string(), hyper::Body::empty())
			.send()
			.await?;

		// Handle a non-success status.
		let response = response.error_for_status()?;

		// Get the response body.
		let body = response.bytes().await?;

		let output = String::from_utf8(body.to_vec())?.parse()?;
		Ok(output)
	}
}
