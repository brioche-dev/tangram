use super::State;
use crate::{expression::Dependency, hash::Hash, util::path_exists};
use anyhow::{anyhow, Context, Result};
use async_recursion::async_recursion;
use futures::FutureExt;
use std::path::{Path, PathBuf};

impl State {
	#[async_recursion]
	#[must_use]
	pub async fn checkout_to_artifacts(&self, artifact_hash: Hash) -> Result<PathBuf> {
		// Get the path.
		let path = self.artifacts_path().join(artifact_hash.to_string());

		// Perform the checkout if necessary.
		if !path_exists(&path).await? {
			// Create a temp path to check out the artifact to.
			let temp_path = self.create_temp_path();

			// Create the callback to create dependency artifact checkouts.
			let dependency_handler =
				{
					let builder = self.lock.upgrade().unwrap();
					move |dependency: &Dependency, path: &Path| {
						let builder = builder.clone();
						let dependency = dependency.clone();
						let path = path.to_owned();
						async move {
							// Get the target by checking out the dependency to the artifacts directory.
							let mut target = builder
							.lock_shared()
							.await?
							.checkout_to_artifacts(dependency.artifact)
							.await
							.context("Failed to check out the dependency to the artifacts directory.")?;

							// Add the dependency path to the target.
							if let Some(dependency_path) = dependency.path {
								target.push(dependency_path);
							}

							// Make the target relative to the symlink path.
							let parent_path = path
								.parent()
								.context("Expected the path to have a parent.")?;
							let target = pathdiff::diff_paths(target, parent_path).context(
								"Could not resolve the symlink target relative to the path.",
							)?;

							// Create the symlink.
							tokio::fs::symlink(target, path)
								.await
								.context("Failed to write the symlink for the dependency.")?;

							Ok::<_, anyhow::Error>(())
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

				// If the error is ENOTEMPTY or EEXIST then we can ignore it because there is already an artifact checkout present.
				Err(error)
					if matches!(error.raw_os_error(), Some(libc::ENOTEMPTY | libc::EEXIST)) => {},

				Err(error) => {
					return Err(anyhow!(error)
						.context("Failed to move the checkout to the artifacts path."));
				},
			};
		}

		Ok(path)
	}
}
