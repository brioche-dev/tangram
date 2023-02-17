use super::Client;
use crate::artifact;
use anyhow::Result;

impl Client {
	pub async fn publish_package(&self, artifact_hash: artifact::Hash) -> Result<()> {
		// Build the URL.
		let mut url = self.url.clone();
		let path = format!("/v1/packages/{artifact_hash}");
		url.set_path(&path);

		// Send the request.
		self.request(reqwest::Method::POST, url)
			.send()
			.await?
			.error_for_status()?;

		Ok(())
	}
}

impl Client {
	pub async fn get_package_version(&self, name: &str, version: &str) -> Result<artifact::Hash> {
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

impl Client {
	pub async fn get_package(&self, name: String) -> Result<Package> {
		// Build the URL.
		let mut url = self.url.clone();
		let path = format!("/v1/packages/{name}");
		url.set_path(&path);

		// Make the request.
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
pub struct SearchResult {
	pub name: String,
}

impl Client {
	pub async fn search_packages(&self, query: &str) -> Result<Vec<SearchResult>> {
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
