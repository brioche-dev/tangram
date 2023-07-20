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

		// Track the current subpath.
		let mut current_subpath = Subpath::empty();

		// Handle each path component.
		for name in path.components() {
			// The artifact must be a directory.
			let Some(directory) = artifact.as_directory() else {
				return Ok(None);
			};

			// Update the current subpath.
			current_subpath = current_subpath.join(name.parse().unwrap());

			// Get the entry. If it doesn't exist, return `None`.
			let Some(block) = directory.entries.get(name).copied() else {
				return Ok(None);
			};

			// Get the artifact.
			artifact = Artifact::get(tg, block)
				.await
				.wrap_err("Failed to get the artifact.")?;

			// If the artifact is a symlink, then resolve it.
			if let Artifact::Symlink(symlink) = &artifact {
				match symlink
					.resolve_from(tg, Some(symlink))
					.await
					.wrap_err("Failed to resolve the symlink.")?
				{
					Some(resolved) => artifact = resolved,
					None => return Ok(None),
				}
			}
		}

		Ok(Some(artifact))
	}
}
