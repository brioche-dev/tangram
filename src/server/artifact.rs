use super::{error::bad_request, full, Incoming, Outgoing, Server};
use crate::{
	artifact::Artifact,
	error::{return_error, Error, Result, WrapErr},
};
use http_body_util::BodyExt;

impl Server {
	pub async fn handle_get_artifact_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "artifacts", hash] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let Ok(hash) = hash.parse() else {
			return Ok(bad_request())
		};

		// Get the artifact.
		let artifact = Artifact::try_get_local(&self.tg, hash).await?;
		let data = artifact.map(|artifact| artifact.to_data());

		// Create the response.
		let body = serde_json::to_vec(&data)
			.map_err(Error::other)
			.wrap_err("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();

		Ok(response)
	}

	pub async fn handle_post_artifact_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read and deserialize the request body.
		let body = request
			.into_body()
			.collect()
			.await
			.map_err(Error::other)
			.wrap_err("Failed to read the request body.")?
			.to_bytes();
		let data = serde_json::from_slice(&body)
			.map_err(Error::other)
			.wrap_err("Failed to deserialize the request body.")?;

		// Create the artifact.
		let artifact = Artifact::add(&self.tg, data).await?;

		// Create the response.
		let body = artifact.hash().to_string();
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();

		Ok(response)
	}
}
