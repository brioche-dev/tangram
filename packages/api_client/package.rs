use crate::ApiClient;
use anyhow::Result;
use tangram_core::hash::Hash;

impl ApiClient {
	pub async fn publish_package(&self, hash: Hash) -> Result<()> {
		// Build the URL.
		let mut url = self.url.clone();
		let path = format!("/v1/packages/{hash}");
		url.set_path(&path);

		// Send the request.
		self.request(reqwest::Method::POST, url)
			.send()
			.await?
			.error_for_status()?;

		Ok(())
	}
}

impl ApiClient {
	pub async fn get_package_version(&self, name: &str, version: &str) -> Result<Hash> {
		// Build the URL.
		let mut url = self.url.clone();
		let path = format!("/v1/packages/{name}/versions/{version}");
		url.set_path(&path);

		// Send the request.
		let response = self
			.request(reqwest::Method::GET, url)
			.send()
			.await?
			.error_for_status()?;

		// Read the response body.
		let response = response.json().await?;

		Ok(response)
	}
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Package {
	pub name: String,
	pub versions: Vec<String>,
}

impl ApiClient {
	pub async fn get_package(&self, name: String) -> Result<Package> {
		// Build the URL.
		let mut url = self.url.clone();
		let path = format!("/v1/packages/{name}");
		url.set_path(&path);

		// Make the request.
		let response = self.request(reqwest::Method::GET, url).send().await?;

		// Handle a non-success status.
		let response = response.error_for_status()?;

		// Read the response body.
		let response = response.json().await?;

		Ok(response)
	}
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SearchResultItem {
	pub name: String,
	pub hash: Hash,
}

impl ApiClient {
	pub async fn search_packages(&self, query: &str) -> Result<Vec<SearchResultItem>> {
		// Build the URL.
		let mut url = self.url.clone();
		url.set_path("/v1/packages/search");
		url.set_query(Some(&format!("query={query}")));

		// Send the request.
		let response = self
			.request(reqwest::Method::GET, url)
			.send()
			.await?
			.error_for_status()?;

		// Read the response body.
		let response = response.json().await?;

		Ok(response)
	}
}
