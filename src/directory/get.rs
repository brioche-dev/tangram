use super::Directory;
use crate::{
	artifact::Artifact,
	error::{Result, WrapErr},
	instance::Instance,
	path::{self, Path},
	return_error,
};

impl Directory {
	pub async fn get(&self, tg: &Instance, path: impl Into<Path>) -> Result<Artifact> {
		let artifact = self
			.try_get(tg, path)
			.await?
			.wrap_err("Failed to get the artifact.")?;
		Ok(artifact)
	}

	pub async fn try_get(&self, tg: &Instance, path: impl Into<Path>) -> Result<Option<Artifact>> {
		// Get the path.
		let path = path.into();
		if let Some(path::Component::Parent) = path.components().first() {
			return_error!("Invalid path.");
		}

		// Track the current artifact.
		let mut artifact = Artifact::Directory(self.clone());

		// Handle each path component.
		for component in path.components() {
			// Get the component name.
			let name = component.as_normal().unwrap();

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
