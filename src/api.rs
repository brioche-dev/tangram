use crate::{Id, Result};
use std::sync::Arc;
use url::Url;

pub struct Client {
	state: Arc<State>,
}

struct State {
	url: Url,
	client: reqwest::Client,
	token: std::sync::RwLock<Option<String>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Login {
	pub id: Id,
	pub url: Url,
	pub token: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SearchResult {
	pub name: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct User {
	pub id: Id,
	pub email: String,
}

impl Client {
	#[must_use]
	pub fn new() -> Self {
		let state = Arc::new(State {
			url: Url::parse("http://localhost:8477").unwrap(),
			client: reqwest::Client::new(),
			token: std::sync::RwLock::new(None),
		});
		Self { state }
	}

	pub async fn create_login(&self) -> Result<Login> {
		let response = self
			.request(reqwest::Method::POST, "/v1/logins")
			.send()
			.await?
			.error_for_status()?;
		let response = response.json().await?;
		Ok(response)
	}

	pub async fn get_login(&self, id: Id) -> Result<Login> {
		let response = self
			.request(reqwest::Method::GET, &format!("/v1/logins/{id}"))
			.send()
			.await?
			.error_for_status()?;
		let response = response.json().await?;
		Ok(response)
	}

	pub async fn publish_package(&self, name: &str) -> Result<()> {
		self.request(reqwest::Method::POST, &format!("/v1/packages/{name}"))
			.send()
			.await?
			.error_for_status()?;
		Ok(())
	}

	pub async fn search_packages(&self, query: &str) -> Result<Vec<SearchResult>> {
		let path = &format!("/v1/packages/search?query={query}");
		let response = self
			.request(reqwest::Method::GET, path)
			.send()
			.await?
			.error_for_status()?;
		let response = response.json().await?;
		Ok(response)
	}

	pub async fn get_current_user(&self) -> Result<User> {
		let response = self
			.request(reqwest::Method::GET, "/v1/user")
			.send()
			.await?
			.error_for_status()?;
		let user = response.json().await?;
		Ok(user)
	}

	fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
		let url = format!("{}{}", self.state.url, path.strip_prefix('/').unwrap());
		let mut request = self.state.client.request(method, url);
		if let Some(token) = self.state.token.read().unwrap().as_ref() {
			request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		request
	}
}
