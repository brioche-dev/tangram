use crate::{blob::Blob, client::Client, hash::Hash};
use anyhow::{Context, Result};
use futures::TryStreamExt;
use tokio_util::io::StreamReader;

impl Client {
	pub async fn add_blob(
		&self,
		reader: Box<dyn tokio::io::AsyncRead + Send + Sync + Unpin>,
		hash: Hash,
	) -> Result<Hash> {
		let stream = tokio_util::io::ReaderStream::new(reader);
		let body = hyper::Body::wrap_stream(stream);

		// Build the URL.
		let path = format!("/blobs/{hash}");
		let mut url = self.url.clone();
		url.set_path(&path);

		// Send the request.
		let response = self
			.create_request(http::Method::POST, url.to_string(), body)
			.send()
			.await?;

		// Handle a non-success status.
		let response = response.error_for_status()?;

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
		let path = format!("/blobs/{hash}");
		let mut url = self.url.clone();
		url.set_path(&path);

		// Send the request.
		let response = self
			.create_request(http::Method::GET, url.to_string(), hyper::Body::empty())
			.send()
			.await
			.context("Failed to send the request")?;

		// Handle a non-success status.
		let response = response.error_for_status()?;

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
