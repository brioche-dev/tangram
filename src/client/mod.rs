use url::Url;

pub mod artifact;
pub mod blob;
pub mod login;
pub mod operation;
pub mod package;
pub mod pull;
pub mod push;
pub mod user;

pub const API_URL: &str = "https://api.tangram.dev";

pub struct Client {
	url: Url,
	token: Option<String>,
	http_client: reqwest::Client,
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
