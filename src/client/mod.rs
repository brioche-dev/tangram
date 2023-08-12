use url::Url;

pub mod block;
pub mod login;
pub mod operation;
pub mod package;
// pub mod pull;
// pub mod push;
pub mod user;

pub const API_URL: &str = "https://api.tangram.dev";

pub struct Client {
	url: Url,
	token: std::sync::RwLock<Option<String>>,
	client: reqwest::Client,
}

impl Client {
	#[must_use]
	pub fn new(url: Url, token: Option<String>) -> Client {
		let client = reqwest::Client::builder()
			.pool_max_idle_per_host(16)
			.build()
			.unwrap();
		Client {
			url,
			token: std::sync::RwLock::new(token),
			client,
		}
	}

	pub fn set_token(&self, token: Option<String>) {
		*self.token.write().unwrap() = token;
	}
}

impl Client {
	pub fn request(&self, method: reqwest::Method, url: Url) -> reqwest::RequestBuilder {
		let mut request = self.client.request(method, url);
		if let Some(token) = self.token.read().unwrap().as_ref() {
			request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		request
	}
}
