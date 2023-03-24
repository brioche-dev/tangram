use super::{error::bad_request, full};
use crate::{
	error::{return_error, Error, Result, WrapErr},
	Instance,
};
use http_body_util::BodyExt;

impl Instance {
	pub async fn handle_add_artifact_request(
		&self,
		request: super::Request,
	) -> Result<super::Response> {
		// Read and deserialize the request body.
		let body = request
			.into_body()
			.collect()
			.await
			.map_err(Error::other)
			.wrap_err("Failed to read the request body.")?
			.to_bytes();
		let artifact = serde_json::from_slice(&body)
			.map_err(Error::other)
			.wrap_err("Failed to deserialize the request body.")?;

		// Add the artifact.
		let outcome = self
			.try_add_artifact(&artifact)
			.await
			.wrap_err("Failed to add the artifact.")?;

		// Create the response.
		let body = serde_json::to_vec(&outcome)
			.map_err(Error::other)
			.wrap_err("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();

		Ok(response)
	}

	#[allow(clippy::unused_async)]
	pub async fn handle_get_artifact_request(
		&self,
		request: super::Request,
	) -> Result<super::Response> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "artifacts", hash] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let Ok(hash) = hash.parse() else { return Ok(bad_request()) };

		// Get the artifact.
		let artifact = self.try_get_artifact_local(hash)?;

		// Create the response.
		let body = serde_json::to_vec(&artifact)
			.map_err(Error::other)
			.wrap_err("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();

		Ok(response)
	}
}
