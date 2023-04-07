use super::Client;
use crate::{
	blob,
	error::{Error, Result, WrapErr},
};
use futures::TryStreamExt;
use tokio::io::AsyncRead;
use tokio_util::io::StreamReader;

impl Client {
	pub async fn add_blob<R>(&self, reader: R, blob_hash: blob::Hash) -> Result<blob::Hash>
	where
		R: AsyncRead + Send + Sync + Unpin + 'static,
	{
		// Build the URL.
		let path = format!("/v1/blobs/{blob_hash}");
		let mut url = self.url.clone();
		url.set_path(&path);

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

impl Client {
	pub async fn get_blob(&self, blob_hash: blob::Hash) -> Result<impl AsyncRead> {
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
		let reader = StreamReader::new(body);

		Ok(reader)
	}
}
