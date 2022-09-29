use crate::{blob::Blob, client::Client, hash::Hash};
use anyhow::{Context, Result};
use futures::TryStreamExt;
use std::path::Path;
use tokio_util::io::StreamReader;

impl Client {
	pub async fn add_blob(&self, path: &Path, hash: Hash) -> Result<Hash> {
		// Create a stream for the file.
		let file = tokio::fs::File::open(&path)
			.await
			.with_context(|| format!(r#"Failed to open file at path "{}"."#, path.display()))?;
		let stream = tokio_util::io::ReaderStream::new(file);
		let request = hyper::Body::wrap_stream(stream);

		// Perform the request.
		let response = self.post(&format!("/blobs/{hash}"), request).await?;

		// Read the response.
		let response = hyper::body::to_bytes(response)
			.await
			.context("Failed to read the response.")?;
		let hash = String::from_utf8(response.to_vec())
			.context("Failed to read the response as UTF-8.")?
			.parse()
			.context("Failed to parse the hash.")?;

		Ok(hash)
	}

	pub async fn get_blob(&self, hash: Hash) -> Result<Blob> {
		// Perform the request.
		let path = format!("/blobs/{hash}");
		let response = self
			.get(&path)
			.await?
			.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error));

		// Create an async reader from the body.
		let body = StreamReader::new(response);

		// Create the blob.
		let blob = Blob::Remote(Box::new(body));

		Ok(blob)
	}
}
