use super::Client;
use crate::{error::Result, rid::Rid};
use url::Url;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Login {
	pub id: Rid,
	pub url: Url,
	pub token: Option<String>,
}

impl Client {
	pub async fn create_login(&self) -> Result<Login> {
		// Send the request.
		let mut url = self.url.clone();
		url.set_path("/v1/logins");
		let response = self
			.request(reqwest::Method::POST, url)
			.send()
			.await?
			.error_for_status()?;

		// Get the response.
		let response = response.json().await?;
		Ok(response)
	}
}

impl Client {
	pub async fn get_login(&self, id: Rid) -> Result<Login> {
		// Send the request.
		let mut url = self.url.clone();
		url.set_path(&format!("/v1/logins/{id}"));
		let response = self
			.request(reqwest::Method::GET, url)
			.send()
			.await?
			.error_for_status()?;

		// Get the response.
		let response = response.json().await?;
		Ok(response)
	}
}
