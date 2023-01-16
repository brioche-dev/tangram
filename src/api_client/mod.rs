use url::Url;

pub mod login;
pub mod package;

#[allow(clippy::module_name_repetitions)]
#[derive(Clone)]
pub struct ApiClient {
	pub url: Url,
	pub token: Option<String>,
	pub http_client: reqwest::Client,
}

impl ApiClient {
	#[must_use]
	pub fn new(url: Url, token: Option<String>) -> ApiClient {
		let http_client = reqwest::Client::new();
		ApiClient {
			url,
			token,
			http_client,
		}
	}
}

impl ApiClient {
	pub fn request(&self, method: reqwest::Method, url: Url) -> reqwest::RequestBuilder {
		let mut request = self.http_client.request(method, url);
		if let Some(token) = &self.token {
			request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		request
	}
}
