use super::Client;
use crate::blob::{Blob, BlobHash};
use anyhow::{Context, Result};
use futures::TryStreamExt;
use tokio_util::io::StreamReader;

impl Client {
	pub async fn add_blob(
		&self,
		reader: Box<dyn tokio::io::AsyncRead + Send + Sync + Unpin>,
		blob_hash: BlobHash,
	) -> Result<BlobHash> {
		let stream = tokio_util::io::ReaderStream::new(reader);
		let body = hyper::Body::wrap_stream(stream);

		// Build the URL.
		let path = format!("/v1/blobs/{blob_hash}");
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
		let blob_hash = String::from_utf8(response.to_vec())
			.context("Failed to read the response as UTF-8.")?
			.parse()
			.context("Failed to parse the blob hash.")?;

		Ok(blob_hash)
	}

	pub async fn get_blob(&self, blob_hash: BlobHash) -> Result<Blob> {
		// Build the URL.
		let path = format!("/v1/blobs/{blob_hash}");
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
		let blob = Box::new(body);

		Ok(blob)
	}
}
