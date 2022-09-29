use crate::{builder, expression::Dependency, hash::Hash, util::path_exists};
use anyhow::{anyhow, Context, Result};
use async_recursion::async_recursion;
use futures::FutureExt;
use std::path::{Path, PathBuf};

impl builder::Shared {
	#[async_recursion]
	#[must_use]
	pub async fn checkout_to_artifacts(&self, artifact_hash: Hash) -> Result<PathBuf> {
		// Get the path.
		let path = self.artifacts_path().join(artifact_hash.to_string());

		// Perform the checkout if necessary.
		if !path_exists(&path).await? {
			// Create a temp path to checkout the artifact to.
			let temp_path = self.create_temp_path();

			// Create the callback to create dependency artifact checkouts.
			let dependency_handler = {
				let server = self.clone();
				move |dependency: &Dependency, path: &Path| {
					let server = server.clone();
					let dependency = dependency.clone();
					let path = path.to_owned();
					async move {
						// Checkout the dependency to an artifact.
						let dependency_path = server
							.checkout_to_artifacts(dependency.artifact)
							.await
							.context(
								"Failed to checkout the dependency to the artifacts directory.",
							)?;

						// Compute the symlink target.
						let parent_path = path
							.parent()
							.ok_or_else(|| anyhow!("Expected the path to have a parent."))?;
						let dependency_path = pathdiff::diff_paths(dependency_path, parent_path)
							.ok_or_else(|| {
								anyhow!(
									"Could not resolve the symlink target relative to the path."
								)
							})?;

						// Create the symlink.
						tokio::fs::symlink(dependency_path, path)
							.await
							.context("Failed to write the symlink for the dependency.")?;

						Ok(())
					}
					.boxed()
				}
			};

			// Perform the checkout.
			self.checkout(artifact_hash, &temp_path, Some(&dependency_handler))
				.await
				.context("Failed to perform the checkout.")?;

			// Move the checkout to the artifacts path.
			match tokio::fs::rename(&temp_path, &path).await {
				Ok(()) => {},

				// If the error is ENOTEMPTY or EEXIST then we can ignore it because there is already a artifact checkout present.
				Err(error)
					if error.raw_os_error() == Some(libc::ENOTEMPTY)
						|| error.raw_os_error() == Some(libc::EEXIST) => {},

				Err(error) => {
					return Err(anyhow::Error::from(error)
						.context("Failed to move the checkout to the artifacts path."));
				},
			};
		}

		Ok(path)
	}
}
