use super::{error::bad_request, Server};
use crate::artifact::{AddArtifactOutcome, Artifact};
use anyhow::{bail, Context, Result};

pub type AddArtifactRequest = Artifact;

pub type AddArtifactResponse = AddArtifactOutcome;

impl Server {
	pub(super) async fn handle_add_artifact_request(
		&self,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read and deserialize the request body.
		let body = hyper::body::to_bytes(request.into_body())
			.await
			.context("Failed to read the request body.")?;
		let artifact =
			serde_json::from_slice(&body).context("Failed to deserialize the request body.")?;

		// Add the artifact.
		let outcome = self
			.cli
			.try_add_artifact(&artifact)
			.await
			.context("Failed to add the artifact.")?;

		// Create the response.
		let body =
			serde_json::to_vec(&outcome).context("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}

	#[allow(clippy::unused_async)]
	pub(super) async fn handle_get_artifact_request(
		&self,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let hash = if let ["v1", "artifacts", hash] = path_components.as_slice() {
			hash
		} else {
			bail!("Unexpected path.");
		};
		let hash = match hash.parse() {
			Ok(hash) => hash,
			Err(_) => return Ok(bad_request()),
		};

		// Get the artifact.
		let artifact = self.cli.try_get_artifact_local(hash)?;

		// Create the response.
		let body =
			serde_json::to_vec(&artifact).context("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}
