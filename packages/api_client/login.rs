use crate::ApiClient;
use anyhow::Result;
use tangram_core::id::Id;
use url::Url;

pub struct User {
	pub id: Id,
	pub email: String,
	pub token: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CreateLoginResponse {
	pub id: Id,
	pub login_url: Url,
}

impl ApiClient {
	pub async fn create_login(&self) -> Result<CreateLoginResponse> {
		// Perform the request.
		let mut url = self.api_url.clone();
		url.set_path("/v1/logins/");
		let response = self
			.client
			.request(reqwest::Method::POST, url)
			.send()
			.await?;
		let response = response.error_for_status()?;

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
	pub async fn get_login(&self, _id: Id) -> Result<GetLoginResponse> {
		// Perform the request.
		let mut url = self.api_url.clone();
		url.set_path("/v1/logins/");
		let response = self
			.client
			.request(reqwest::Method::GET, url)
			.send()
			.await?;
		let response = response.error_for_status()?;

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
		// Perform the request.
		let mut url = self.api_url.clone();
		url.set_path("/v1/user/");
		let response = self
			.client
			.request(reqwest::Method::GET, url)
			.header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token))
			.send()
			.await?;
		let response = response.error_for_status()?;

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
