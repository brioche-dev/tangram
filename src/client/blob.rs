use super::Client;
use crate::{
	blob,
	error::{Error, Result, WrapErr},
};
use futures::TryStreamExt;
use tokio::io::AsyncRead;
use tokio_util::io::StreamReader;

impl Client {
	pub async fn try_get_blob(&self, blob_hash: blob::Hash) -> Result<Option<impl AsyncRead>> {
		// Build the URL.
		let path = format!("/v1/blobs/{blob_hash}");
		let mut url = self.url.clone();
		url.set_path(&path);

		// Send the request.
		let response = self.request(http::Method::GET, url).send().await?;

		// Check if the blob exists.
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}

		// Check if the request was successful.
		let response = response.error_for_status()?;

		// Get the response body.
		let body = response
			.bytes_stream()
			.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error));

		// Create a reader for the response body.
		let reader = StreamReader::new(body);

		Ok(Some(reader))
	}

	pub async fn post_blob<R>(&self, reader: R) -> Result<blob::Hash>
	where
		R: AsyncRead + Send + Sync + Unpin + 'static,
	{
		// Build the URL.
		let path = "/v1/blobs";
		let mut url = self.url.clone();
		url.set_path(path);

		// Create the body.
		let body = reqwest::Body::wrap_stream(tokio_util::io::ReaderStream::new(reader));

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
			.map_err(Error::other)
			.wrap_err("Failed to read the response as UTF-8.")?
			.parse()
			.map_err(Error::other)
			.wrap_err("Failed to parse the blob hash.")?;

		Ok(blob_hash)
	}
}
