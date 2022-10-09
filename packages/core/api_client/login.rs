use super::ApiClient;
use crate::id::Id;
use anyhow::Result;
use url::Url;

pub struct User {
	pub id: Id,
	pub email: String,
	pub token: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CreateLoginResponse {
	pub id: Id,
	pub login_page_url: Url,
}

impl ApiClient {
	pub async fn create_login(&self) -> Result<CreateLoginResponse> {
		// Send the request.
		let mut url = self.url.clone();
		url.set_path("/v1/logins/");
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct GetLoginResponse {
	pub id: Id,
	pub token: Option<String>,
}

impl ApiClient {
	pub async fn get_login(&self, id: Id) -> Result<GetLoginResponse> {
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct GetCurrentUserResponse {
	pub id: Id,
	pub email: String,
	pub token: String,
}

impl ApiClient {
	pub async fn get_current_user(&self, token: String) -> Result<User> {
		// Send the request.
		let mut url = self.url.clone();
		url.set_path("/v1/user");
		let response = self
			.request(reqwest::Method::GET, url)
			.header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token))
			.send()
			.await?
			.error_for_status()?;

		// Get the response.
		let response: GetCurrentUserResponse = response.json().await?;
		let user = User {
			id: response.id,
			email: response.email,
			token,
		};
		Ok(user)
	}
}
