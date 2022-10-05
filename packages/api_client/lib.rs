use url::Url;

pub mod login;
pub mod package;

pub struct ApiClient {
	pub url: Url,
	pub token: Option<String>,
	pub client: tangram_core::client::Client,
	pub http_client: reqwest::Client,
}

impl ApiClient {
	#[must_use]
	pub fn new(url: Url, token: Option<String>) -> ApiClient {
		let http_client = reqwest::Client::new();
		let client = tangram_core::client::Client::new(url.clone(), token.clone());
		ApiClient {
			url,
			token,
			client,
			http_client,
		}
	}

	pub fn request(&self, method: reqwest::Method, url: Url) -> reqwest::RequestBuilder {
		let mut request = self.http_client.request(method, url);
		if let Some(token) = &self.token {
			request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		request
	}
}
