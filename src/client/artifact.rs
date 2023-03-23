use super::Client;
use crate::{
	artifact::{self, Artifact},
	error::{return_error, Error, Result, WrapErr},
};

impl Client {
	pub async fn try_get_artifact(
		&self,
		artifact_hash: artifact::Hash,
	) -> Result<Option<Artifact>> {
		// Get a permit.
		let _permit = self
			.socket_semaphore
			.acquire()
			.await
			.map_err(Error::other)?;

		// Create the path.
		let path = format!("/v1/artifacts/{artifact_hash}");

		// Build the URL.
		let mut url = self.url.clone();
		url.set_path(&path);

		// Send the request.
		let response = self
			.request(http::Method::GET, url)
			.send()
			.await?
			.error_for_status()?;

		// Read the response body.
		let response = response
			.json()
			.await
			.wrap_err("Failed to read the response body.")?;

		Ok(response)
	}
}

impl Client {
	pub async fn try_add_artifact(&self, artifact: &Artifact) -> Result<artifact::add::Outcome> {
		// Get a permit.
		let _permit = self
			.socket_semaphore
			.acquire()
			.await
			.map_err(Error::other)?;

		// Build the URL.
		let mut url = self.url.clone();
		url.set_path("/v1/artifacts/");

		// Send the request.
		let response = self
			.request(http::Method::POST, url)
			.json(&artifact)
			.send()
			.await?
			.error_for_status()?;

		// Read the response body.
		let response = response
			.json()
			.await
			.wrap_err("Failed to read the response body.")?;

		Ok(response)
	}

	pub async fn add_artifact(&self, artifact: &Artifact) -> Result<artifact::Hash> {
		if let artifact::add::Outcome::Added { artifact_hash } =
			self.try_add_artifact(artifact).await?
		{
			Ok(artifact_hash)
		} else {
			return_error!("Failed to add the artifact.")
		}
	}
}
