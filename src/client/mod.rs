use std::sync::Arc;
use tokio::sync::Semaphore;
use url::Url;

use crate::Cli;

mod artifact;
mod blob;

impl Cli {
	#[must_use]
	pub fn create_client(&self, url: Url, token: Option<String>) -> Client {
		Client::new(url, token, Arc::clone(&self.inner.socket_semaphore))
	}
}

pub struct Client {
	pub url: Url,
	pub token: Option<String>,
	pub socket_semaphore: Arc<Semaphore>,
	pub http_client: reqwest::Client,
}

impl Client {
	#[must_use]
	pub fn new(url: Url, token: Option<String>, socket_semaphore: Arc<Semaphore>) -> Client {
		let http_client = reqwest::Client::new();
		Client {
			url,
			token,
			socket_semaphore,
			http_client,
		}
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
