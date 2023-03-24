use super::{error::bad_request, error::not_found, full};
use crate::{
	blob,
	error::{return_error, Result},
	util::http::BodyStream,
	Instance,
};
use futures::TryStreamExt;
use http_body_util::{BodyExt, StreamBody};

impl Instance {
	pub async fn handle_add_blob_request(
		&self,
		request: super::Request,
	) -> Result<super::Response> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "blobs", blob_hash] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		if blob_hash.parse::<blob::Hash>().is_err() {
			return Ok(bad_request());
		}

		// Create an async reader from the body.
		let body = tokio_util::io::StreamReader::new(
			BodyStream::new(request.into_body())
				.try_filter_map(|frame| Box::pin(async move { Ok(frame.into_data().ok()) }))
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error)),
		);

		// Add the blob.
		let hash = self.add_blob(body).await?;

		// Create the response.
		let body = hash.to_string();
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();

		Ok(response)
	}

	pub async fn handle_get_blob_request(
		&self,
		request: super::Request,
	) -> Result<super::Response> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "blobs", blob_hash] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let blob_hash: blob::Hash = match blob_hash.parse() {
			Ok(client_blob_hash) => client_blob_hash,
			Err(_) => return Ok(bad_request()),
		};

		// Get the blob.
		let Some(file) = self.try_get_blob(blob_hash).await? else { return Ok(not_found()) };

		// Create the stream for the file.
		let body = StreamBody::new(
			tokio_util::io::ReaderStream::new(file)
				.map_ok(hyper::body::Frame::data)
				.map_err(Into::into),
		);

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(body.boxed())
			.unwrap();

		Ok(response)
	}
}
