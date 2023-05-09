use super::Directory;
use crate::{
	artifact::Artifact,
	error::{Result, WrapErr},
	instance::Instance,
	path::Subpath,
};

impl Directory {
	pub async fn get(&self, tg: &Instance, path: &Subpath) -> Result<Artifact> {
		let artifact = self
			.try_get(tg, path)
			.await?
			.wrap_err("Failed to get the artifact.")?;
		Ok(artifact)
	}

	pub async fn try_get(&self, tg: &Instance, path: &Subpath) -> Result<Option<Artifact>> {
		// Track the current artifact.
		let mut artifact = Artifact::Directory(self.clone());

		// Handle each path component.
		for name in path.components() {
			// The artifact must be a directory.
			let Some(directory) = artifact.as_directory() else {
				return Ok(None);
			};

			// Get the entry hash for the file name. If it doesn't exist, return `None`.
			let Some(artifact_hash) = directory.entries.get(name).copied() else {
				return Ok(None);
			};

			// Get the artifact.
			artifact = Artifact::get(tg, artifact_hash)
				.await
				.wrap_err("Failed to get the artifact.")?;
		}

		Ok(Some(artifact))
	}
}
