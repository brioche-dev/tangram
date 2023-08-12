use super::Client;
use crate::{error::Result, package::Package};

impl Client {
	pub async fn publish_package(&self, package: Package) -> Result<()> {
		// Build the URL.
		let id = package.id();
		let mut url = self.url.clone();
		let path = format!("/v1/packages/{id}");
		url.set_path(&path);

		// Send the request.
		self.request(reqwest::Method::POST, url)
			.send()
			.await?
			.error_for_status()?;

		Ok(())
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
