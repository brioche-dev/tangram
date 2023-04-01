use std::sync::Arc;
use tokio::sync::Semaphore;
use url::Url;

pub struct Client {
	pub url: Url,
	pub token: Option<String>,
	pub socket_semaphore: Arc<Semaphore>,
	pub http_client: reqwest::Client,
	pub instance_client: crate::client::Client,
}

impl Client {
	#[must_use]
	pub fn new(url: Url, token: Option<String>, socket_semaphore: Arc<Semaphore>) -> Client {
		let http_client = reqwest::Client::new();
		let instance_client =
			crate::client::Client::new(url.clone(), token.clone(), Arc::clone(&socket_semaphore));
		Client {
			url,
			token,
			socket_semaphore,
			http_client,
			instance_client,
		}
	}

	#[must_use]
	pub fn instance_client(&self) -> &crate::client::Client {
		&self.instance_client
	}
}

impl Client {
	pub fn request(&self, method: reqwest::Method, url: Url) -> reqwest::RequestBuilder {
		let mut request = self.http_client.request(method, url);
		if let Some(token) = &self.token {
			request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		request
	}
}
