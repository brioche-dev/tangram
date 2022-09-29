use anyhow::{bail, Context, Result};
use url::Url;

mod blob;
mod evaluate;
mod expression;
mod package;

#[derive(Clone)]
pub struct Client {
	pub url: Url,
	pub token: Option<String>,
	pub client:
		hyper::Client<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>, hyper::Body>,
}

impl Client {
	#[must_use]
	pub fn new(url: Url, token: Option<String>) -> Client {
		let client: hyper::Client<_, hyper::Body> = hyper::Client::builder().build(
			hyper_rustls::HttpsConnectorBuilder::new()
				.with_native_roots()
				.https_or_http()
				.enable_http1()
				.build(),
		);
		Client { url, token, client }
	}
}

impl Client {
	pub fn create_request(
		&self,
		method: http::Method,
		uri: String,
		body: hyper::Body,
	) -> Result<http::Request<hyper::Body>> {
		let mut request = http::Request::builder().method(method).uri(uri);
		if let Some(token) = &self.token {
			request.headers_mut().unwrap().insert(
				http::header::AUTHORIZATION,
				format!("Bearer {token}").parse().unwrap(),
			);
		}
		let request = request.body(body).unwrap();
		Ok(request)
	}

	pub async fn request(
		&self,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		self.client
			.request(request)
			.await
			.context("Failed to send the request.")
	}

	pub async fn get(&self, path: &str) -> Result<hyper::Body> {
		// Build the URL.
		let mut url = self.url.clone();
		url.set_path(path);

		// Create the request.
		let request =
			self.create_request(http::Method::GET, url.to_string(), hyper::Body::empty())?;

		// Send the request.
		let response = self.request(request).await?;

		// Handle a non-success status.
		if !response.status().is_success() {
			let status = response.status();
			let body = hyper::body::to_bytes(response.into_body())
				.await
				.context("Failed to read response body.")?;
			let body = String::from_utf8(body.to_vec())
				.context("Failed to read response body as string.")?;
			bail!("{status}\n{body}");
		}

		Ok(response.into_body())
	}

	pub async fn get_json<U>(&self, path: &str) -> Result<U>
	where
		U: serde::de::DeserializeOwned,
	{
		// Build the URL.
		let mut url = self.url.clone();
		url.set_path(path);

		// Create the request.
		let request = http::Request::builder()
			.method(http::Method::GET)
			.uri(url.to_string())
			.body(hyper::Body::empty())
			.unwrap();

		// Send the request.
		let response = self
			.client
			.request(request)
			.await
			.context("Failed to send the request.")?;

		// Handle a non-success status.
		if !response.status().is_success() {
			let status = response.status();
			let body = hyper::body::to_bytes(response.into_body())
				.await
				.context("Failed to read response body.")?;
			let body = String::from_utf8(body.to_vec())
				.context("Failed to read response body as string.")?;
			bail!("{status}\n{body}");
		}

		// Read the response body.
		let body = hyper::body::to_bytes(response.into_body())
			.await
			.context("Failed to read response body.")?;

		// Deserialize the response body.
		let response =
			serde_json::from_slice(&body).context("Failed to deserialize the response body.")?;

		Ok(response)
	}

	pub async fn post(&self, path: &str, body: hyper::Body) -> Result<hyper::Body> {
		// Build the URL.
		let mut url = self.url.clone();
		url.set_path(path);

		// Create the request.
		let request = http::Request::builder()
			.method(http::Method::POST)
			.uri(url.to_string())
			.body(body)
			.unwrap();

		// Send the request.
		let response = self.request(request).await?;

		// Handle a non-success status.
		if !response.status().is_success() {
			let status = response.status();
			let body = hyper::body::to_bytes(response.into_body())
				.await
				.context("Failed to read response body.")?;
			let body = String::from_utf8(body.to_vec())
				.context("Failed to read response body as string.")?;
			bail!("{status}\n{body}");
		}

		Ok(response.into_body())
	}

	pub async fn post_json<T, U>(&self, path: &str, body: &T) -> Result<U>
	where
		T: serde::Serialize,
		U: serde::de::DeserializeOwned,
	{
		// Build the URL.
		let mut url = self.url.clone();
		url.set_path(path);

		// Serialize the body.
		let body = serde_json::to_string(&body).context("Failed to serialize the request body.")?;

		// Create the request.
		let request = http::Request::builder()
			.method(http::Method::POST)
			.uri(url.to_string())
			.header(http::header::CONTENT_TYPE, "application/json")
			.body(hyper::Body::from(body))
			.unwrap();

		// Send the request.
		let response = self.request(request).await?;

		// Handle a non-success status.
		if !response.status().is_success() {
			let status = response.status();
			let body = hyper::body::to_bytes(response.into_body())
				.await
				.context("Failed to read response body.")?;
			let body = String::from_utf8(body.to_vec())
				.context("Failed to read response body as string.")?;
			bail!("{status}\n{body}");
		}

		// Read the response body.
		let body = hyper::body::to_bytes(response.into_body())
			.await
			.context("Failed to read response body.")?;

		// Deserialize the response body.
		let response =
			serde_json::from_slice(&body).context("Failed to deserialize the response body.")?;

		Ok(response)
	}
}
