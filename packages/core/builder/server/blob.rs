use super::{error::bad_request, error::not_found, Server};
use crate::hash::Hash;
use anyhow::{bail, Result};
use futures::TryStreamExt;

impl Server {
	pub(super) async fn handle_add_blob_request(
		&self,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let blob_hash = if let ["blobs", blob_hash] = path_components.as_slice() {
			blob_hash
		} else {
			bail!("Unexpected path.")
		};
		let _blob_hash: Hash = match blob_hash.parse() {
			Ok(client_blob_hash) => client_blob_hash,
			Err(_) => return Ok(bad_request()),
		};

		// Create an async reader from the body.
		let body = tokio_util::io::StreamReader::new(
			request
				.into_body()
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error)),
		);

		// Add the blob.
		let hash = self.builder.lock_shared().await?.add_blob(body).await?;

		// Create the response.
		let response = hash.to_string();
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(response))
			.unwrap();

		Ok(response)
	}

	pub(super) async fn handle_get_blob_request(
		&self,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let blob_hash = if let ["blobs", blob_hash] = path_components.as_slice() {
			blob_hash
		} else {
			bail!("Unexpected path.")
		};
		let blob_hash: Hash = match blob_hash.parse() {
			Ok(client_blob_hash) => client_blob_hash,
			Err(_) => return Ok(bad_request()),
		};

		// Get the blob.
		let file = match self
			.builder
			.lock_shared()
			.await?
			.try_get_blob(blob_hash)
			.await?
		{
			Some(path) => path,
			None => return Ok(not_found()),
		};

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
