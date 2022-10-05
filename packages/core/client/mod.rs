use url::Url;

mod blob;
mod evaluate;
mod expression;

#[derive(Clone)]
pub struct Client {
	pub url: Url,
	pub token: Option<String>,
	pub client: reqwest::Client,
}

impl Client {
	#[must_use]
	pub fn new(url: Url, token: Option<String>) -> Client {
		let client = reqwest::Client::new();
		Client { url, token, client }
	}
}

impl Client {
	pub fn create_request(
		&self,
		method: reqwest::Method,
		uri: String,
		body: hyper::Body,
	) -> reqwest::RequestBuilder {
		let mut request = self.client.request(method, uri);
		if let Some(token) = &self.token {
			request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		request.body(body)
	}
}
