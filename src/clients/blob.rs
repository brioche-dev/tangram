use crate::{blob::Blob, hash::Hash};
use anyhow::{Context, Result};
use futures::TryStreamExt;
use tokio_util::io::StreamReader;
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
	pub async fn add_blob(
		&self,
		reader: Box<dyn tokio::io::AsyncRead + Send + Sync + Unpin>,
		hash: Hash,
	) -> Result<Hash> {
		let stream = tokio_util::io::ReaderStream::new(reader);
		let body = hyper::Body::wrap_stream(stream);

		// Build the URL.
		let path = format!("/v1/blobs/{hash}");
		let mut url = self.url.clone();
		url.set_path(&path);

		// Send the request.
		let response = self
			.request(http::Method::POST, url)
			.body(body)
			.send()
			.await?
			.error_for_status()?;

		// Read the response.
		let response = response.bytes().await?;
		let hash = String::from_utf8(response.to_vec())
			.context("Failed to read the response as UTF-8.")?
			.parse()
			.context("Failed to parse the hash.")?;

		Ok(hash)
	}

	pub async fn get_blob(&self, hash: Hash) -> Result<Blob> {
		// Build the URL.
		let path = format!("/v1/blobs/{hash}");
		let mut url = self.url.clone();
		url.set_path(&path);

		// Send the request.
		let response = self
			.request(http::Method::GET, url)
			.send()
			.await?
			.error_for_status()?;

		// Read the response body.
		let body = response
			.bytes_stream()
			.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error));

		// Create an async reader from the body.
		let body = StreamReader::new(body);

		// Create the blob.
		let blob = Blob::Remote(Box::new(body));

		Ok(blob)
	}
}
