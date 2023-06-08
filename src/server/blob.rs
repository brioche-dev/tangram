use super::{
	error::{bad_request, not_found},
	full, BodyStream, Incoming, Outgoing, Server, StreamBody,
};
use crate::{
	blob::Blob,
	error::{return_error, Result, WrapErr},
};
use futures::TryStreamExt;
use http_body_util::BodyExt;

impl Server {
	pub async fn handle_get_blob_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "blobs", hash] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let Ok(hash) = hash.parse() else {
			return Ok(bad_request());
		};

		// Get the blob reader.
		let blob = Blob::from_hash(hash);
		let Some(reader) = blob.try_get(&self.tg).await? else {
			return Ok(not_found());
		};

		// Create the body.
		let body = StreamBody::new(
			tokio_util::io::ReaderStream::new(reader)
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

	pub async fn handle_post_blob_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Create a reader from the body.
		let body = tokio_util::io::StreamReader::new(
			BodyStream::new(request.into_body())
				.try_filter_map(|frame| Box::pin(async move { Ok(frame.into_data().ok()) }))
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error)),
		);

		// Create the blob.
		let blob = Blob::new(&self.tg, body)
			.await
			.wrap_err("Failed to create the blob.")?;

		// Create the response.
		let body = blob.hash().to_string();
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();

		Ok(response)
	}
}
