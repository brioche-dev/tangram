use super::Client;
use crate::{
	error::{Error, Result},
	id::Id,
};
use url::Url;

pub struct User {
	pub id: Id,
	pub email: String,
	pub token: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Login {
	pub id: Id,
	pub url: Url,
	pub token: Option<String>,
}

impl Client {
	pub async fn create_login(&self) -> Result<Login> {
		// Get a permit.
		let _permit = self
			.socket_semaphore
			.acquire()
			.await
			.map_err(Error::other)?;

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

impl Client {
	pub async fn get_login(&self, id: Id) -> Result<Login> {
		// Get a permit.
		let _permit = self
			.socket_semaphore
			.acquire()
			.await
			.map_err(Error::other)?;

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
struct GetCurrentUserResponse {
	id: Id,
	email: String,
	token: String,
}

impl Client {
	pub async fn get_current_user(&self, token: String) -> Result<User> {
		// Get a permit.
		let _permit = self
			.socket_semaphore
			.acquire()
			.await
			.map_err(Error::other)?;

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
		let response: GetCurrentUserResponse = response.json().await?;
		let user = User {
			id: response.id,
			email: response.email,
			token,
		};
		Ok(user)
	}
}
