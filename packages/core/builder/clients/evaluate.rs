use crate::hash::Hash;
use anyhow::Result;
use url::Url;

#[derive(Clone)]
pub struct Client {
	pub url: Url,
	pub token: Option<String>,
	pub http_client: reqwest::Client,
}

impl Client {
	#[must_use]
	pub fn new(url: Url, token: Option<String>) -> Client {
		let http_client = reqwest::Client::new();
		Client {
			url,
			token,
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

impl Client {
	pub async fn evaluate(&self, hash: Hash) -> Result<Hash> {
		// Build the URL.
		let path = format!("/v1/expressions/{hash}/evaluate");
		let mut url = self.url.clone();
		url.set_path(&path);

		// Send the request.
		let response = self
			.request(http::Method::POST, url)
			.send()
			.await?
			.error_for_status()?;

		// Get the response body.
		let body = response.bytes().await?;

		let output = String::from_utf8(body.to_vec())?.parse()?;
		Ok(output)
	}
}
