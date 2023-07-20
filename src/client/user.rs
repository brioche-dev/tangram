use super::Client;
use crate::{error::Result, rid::Rid};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct User {
	pub id: Rid,
	pub email: String,
}

impl Client {
	pub async fn get_current_user(&self) -> Result<User> {
		// Send the request.
		let mut url = self.url.clone();
		url.set_path("/v1/user");
		let response = self
			.request(reqwest::Method::GET, url)
			.send()
			.await?
			.error_for_status()?;

		// Get the response.
		let user = response.json().await?;

		Ok(user)
	}
}
