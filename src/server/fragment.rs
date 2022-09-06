use crate::{artifact::Artifact, object::Dependency, server::Server, util::path_exists};
use anyhow::{anyhow, Context, Result};
use async_recursion::async_recursion;
use futures::FutureExt;
use std::{
	path::{Path, PathBuf},
	sync::Arc,
};

#[derive(Clone, Copy, Debug)]
pub struct Fragment {
	artifact: Artifact,
}

impl Fragment {
	#[must_use]
	pub fn artifact(&self) -> &Artifact {
		&self.artifact
	}
}

impl Server {
	#[async_recursion]
	#[must_use]
	pub async fn create_fragment(self: &Arc<Self>, artifact: Artifact) -> Result<Fragment> {
		// Get the fragment path.
		let fragment_path = self.path().join("fragments").join(artifact.to_string());

		// Perform the checkout if necessary.
		if !path_exists(&fragment_path).await? {
			// Create a temp to check out the artifact to.
			let temp = self
				.create_temp()
				.await
				.context("Failed to create the temp.")?;
			let temp_path = self.temp_path(&temp);

			// Create the callback to create dependency fragments.
			let dependency_handler = {
				let server = Arc::clone(self);
				move |dependency: &Dependency, path: &Path| {
					let server = Arc::clone(&server);
					let dependency = dependency.clone();
					let path = path.to_owned();
					async move {
						// Checkout the dependency to a fragment.
						let dependency_fragment = server
							.create_fragment(dependency.artifact)
							.await
							.context("Failed to checkout the dependency to a fragment.")?;

						// Get the dependency fragment's path.
						let dependency_fragment_path = server.fragment_path(&dependency_fragment);

						// Compute the symlink target.
						let parent_path = path
							.parent()
							.ok_or_else(|| anyhow!("Expected the path to have a parent."))?;
						let dependency_path =
							pathdiff::diff_paths(dependency_fragment_path, parent_path)
								.ok_or_else(|| {
									anyhow!("Could not resolve the symlink target relative to the path.")
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
			self.checkout(artifact, &temp_path, Some(&dependency_handler))
				.await
				.context("Failed to perform the checkout.")?;

			// Move the checkout to the fragments path.
			tokio::fs::rename(&temp_path, &fragment_path)
				.await
				.context("Failed to move the checkout to the fragment path.")?;
		}

		Ok(Fragment { artifact })
	}

	#[must_use]
	pub fn fragment_path(self: &Arc<Self>, fragment: &Fragment) -> PathBuf {
		self.path()
			.join("fragments")
			.join(fragment.artifact().to_string())
	}
}
