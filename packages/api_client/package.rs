use crate::ApiClient;
use anyhow::Result;
use tangram_core::{expression, hash::Hash};

impl ApiClient {
	pub async fn publish_package(&self, hash: Hash) -> Result<()> {
		// Build the URL.
		let mut url = self.api_url.clone();
		let path = format!("/v1/packages/{hash}");
		url.set_path(&path);

		// Make the request.
		self.client
			.request(reqwest::Method::POST, url)
			.send()
			.await?;

		Ok(())
	}
}

impl ApiClient {
	pub async fn get_package(&self, name: String, version: String) -> Result<expression::Package> {
		// Build the URL.
		let mut url = self.api_url.clone();
		let path = format!("/v1/packages/{name}/{version}");
		url.set_path(&path);

		// Make the request.
		let response = self
			.client
			.request(reqwest::Method::GET, url)
			.send()
			.await?;

		// Handle a non-success status.
		let response = response.error_for_status()?;

		// Read the response body.
		let response = response.json().await?;

		Ok(response)
	}
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SearchResult {
	name: String,
}

impl ApiClient {
	pub async fn search_packages(&self, query: String) -> Result<Vec<SearchResult>> {
		// Build the URL.
		let mut url = self.api_url.clone();
		url.set_path("/v1/packages/search");
		url.set_query(Some(&format!("query={query}")));

		// Make the request.
		let response = self
			.client
			.request(reqwest::Method::GET, url)
			.send()
			.await?;

		// Handle a non-success status.
		let response = response.error_for_status()?;

		// Read the response body.
		let response = response.json().await?;

		Ok(response)
	}
}
