use super::{error::bad_request, error::not_found};
use crate::{blob, Instance};
use anyhow::{bail, Result};
use futures::TryStreamExt;

impl Instance {
	pub async fn handle_add_blob_request(
		&self,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "blobs", blob_hash] = path_components.as_slice() else {
			bail!("Unexpected path.")
		};
		if blob_hash.parse::<blob::Hash>().is_err() {
			return Ok(bad_request());
		}

		// Create an async reader from the body.
		let body = tokio_util::io::StreamReader::new(
			request
				.into_body()
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error)),
		);

		// Add the blob.
		let hash = self.add_blob(body).await?;

		// Create the response.
		let response = hash.to_string();
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(response))
			.unwrap();

		Ok(response)
	}

	pub async fn handle_get_blob_request(
		&self,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "blobs", blob_hash] = path_components.as_slice() else {
			bail!("Unexpected path.")
		};
		let blob_hash: blob::Hash = match blob_hash.parse() {
			Ok(client_blob_hash) => client_blob_hash,
			Err(_) => return Ok(bad_request()),
		};

		// Get the blob.
		let Some(file) = self.try_get_blob(blob_hash).await? else { return Ok(not_found()) };

		// Create the stream for the file.
		let stream = tokio_util::io::ReaderStream::new(file);
		let response = hyper::Body::wrap_stream(stream);

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(response)
			.unwrap();

		Ok(response)
	}
}
