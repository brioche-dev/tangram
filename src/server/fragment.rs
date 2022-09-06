use crate::{artifact::Artifact, object::Dependency, server::Server, util::path_exists};
use anyhow::{Context, Result};
use async_recursion::async_recursion;
use futures::FutureExt;
use std::{path::PathBuf, sync::Arc};

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
			let path_for_dependency = {
				let server = Arc::clone(self);
				move |dependency: &Dependency| {
					let server = Arc::clone(&server);
					let dependency = dependency.clone();
					async move {
						let dependency_fragment =
							server.create_fragment(dependency.artifact).await?;
						let dependency_fragment_path = server.fragment_path(&dependency_fragment);
						Ok(Some(dependency_fragment_path))
					}
					.boxed()
				}
			};

			// Perform the checkout.
			self.checkout(artifact, &temp_path, Some(&path_for_dependency))
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
