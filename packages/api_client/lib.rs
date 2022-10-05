use url::Url;

pub mod login;
pub mod package;

pub struct ApiClient {
	api_url: Url,
	token: String,
	client: reqwest::Client,
}

impl ApiClient {
	#[must_use]
	pub fn new(api_url: Url, token: String) -> ApiClient {
		let client = reqwest::Client::new();
		ApiClient {
			api_url,
			token,
			client,
		}
	}
}
