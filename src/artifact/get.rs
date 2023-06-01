use super::{Artifact, Hash};
use crate::{
	error::{Result, WrapErr},
	instance::Instance,
};
use async_recursion::async_recursion;

impl Artifact {
	#[async_recursion]
	pub async fn get(tg: &'async_recursion Instance, hash: Hash) -> Result<Self> {
		let artifact = Self::try_get(tg, hash)
			.await?
			.wrap_err_with(|| format!(r#"Failed to get the artifact with hash "{hash}"."#))?;
		Ok(artifact)
	}

	pub async fn try_get(tg: &Instance, hash: Hash) -> Result<Option<Self>> {
		// Attempt to get the artifact from the database.
		if let Some(artifact) = Self::try_get_local(tg, hash).await? {
			return Ok(Some(artifact));
		}

		// // Attempt to get the artifact from the API.
		// let artifact = tg
		// 	.api_instance_client()
		// 	.try_get(artifact_hash)
		// 	.await
		// 	.ok()
		// 	.flatten();
		// if let Some(artifact) = artifact {
		// 	return Ok(Some(artifact));
		// }

		Ok(None)
	}

	pub async fn get_local(tg: &Instance, hash: Hash) -> Result<Self> {
		let artifact = Self::try_get_local(tg, hash)
			.await?
			.wrap_err_with(|| format!(r#"Failed to find the artifact with hash "{hash}"."#))?;
		Ok(artifact)
	}

	pub async fn try_get_local(tg: &Instance, hash: Hash) -> Result<Option<Self>> {
		// Get the artifact data from the database.
		let Some(data) = tg.database.try_get_artifact(hash)? else {
			return Ok(None);
		};

		// Create the artifact from the serialized artifact.
		let artifact = Self::from_data(tg, hash, data).await?;

		Ok(Some(artifact))
	}
}
