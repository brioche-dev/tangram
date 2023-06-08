use super::Client;
use crate::{error::Result, id::Id};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct User {
	pub id: Id,
	pub email: String,
	pub token: String,
}

impl Client {
	pub async fn get_current_user(&self, token: String) -> Result<User> {
		// Send the request.
		let mut url = self.url.clone();
		url.set_path("/v1/user");
		let response = self
			.request(reqwest::Method::GET, url)
			.bearer_auth(token.clone())
			.send()
			.await?
			.error_for_status()?;

		// Get the response.
		let user = response.json().await?;

		Ok(user)
	}
}
