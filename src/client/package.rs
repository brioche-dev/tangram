use super::Client;
use crate::hash::Hash;
use anyhow::{bail, Context, Result};

impl Client {
	pub async fn search(&self, name: &str) -> Result<Vec<String>> {
		// Build the URL.
		let mut url = self.url.clone();
		url.set_path("/packages");
		url.set_query(Some(&format!("name={name}")));

		// Create the request.
		let request = http::Request::builder()
			.method(http::Method::GET)
			.uri(url.to_string())
			.body(hyper::Body::empty())
			.unwrap();

		// Send the request.
		let response = self
			.request(request)
			.await
			.context("Failed to send the request.")?;

		// Handle a non-success status.
		if !response.status().is_success() {
			let status = response.status();
			let body = hyper::body::to_bytes(response.into_body())
				.await
				.context("Failed to read response body.")?;
			let body = String::from_utf8(body.to_vec())
				.context("Failed to read response body as string.")?;
			bail!("{status}\n{body}");
		}

		// Read the response body.
		let body = hyper::body::to_bytes(response.into_body())
			.await
			.context("Failed to read response body.")?;

		// Deserialize the response body.
		let response =
			serde_json::from_slice(&body).context("Failed to deserialize the response body.")?;

		Ok(response)
	}

	// Retrieve the package with the given name and version.
	pub async fn get_package(&self, name: &str, version: &str) -> Result<Option<Hash>> {
		let path = format!("/packages/{name}/versions/{version}");
		let artifact = self.get_json(&path).await?;
		Ok(artifact)
	}

	pub async fn publish_package(&self, hash: Hash) -> Result<()> {
		let path = format!("/packages/{hash}");
		let _response = self.post(&path, hyper::Body::empty()).await?;
		Ok(())
	}
}
