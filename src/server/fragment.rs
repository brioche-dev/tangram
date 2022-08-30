use crate::{
	artifact::Artifact, fragment::Fragment, object::Dependency, server::Server, util::path_exists,
};
use anyhow::Result;
use async_recursion::async_recursion;
use futures::FutureExt;
use std::sync::Arc;

impl Server {
	#[async_recursion]
	#[must_use]
	pub async fn create_fragment(self: &Arc<Self>, artifact: &Artifact) -> Result<Fragment> {
		// Get the path to the fragment.
		let fragment_path = self.path.join("fragments").join(artifact.to_string());

		// If the fragment path does not exist, then checkout the object to the fragment path.
		if !path_exists(&fragment_path).await? {
			let path_for_dependency = {
				let server = Arc::clone(self);
				move |dependency: &Dependency| {
					let server = Arc::clone(&server);
					let dependency = dependency.clone();
					async move {
						let dependency_fragment =
							server.create_fragment(&dependency.artifact).await?;
						Ok(Some(dependency_fragment.path()))
					}
					.boxed()
				}
			};
			self.checkout(artifact, &fragment_path, Some(&path_for_dependency))
				.await?;
		}

		// Create the fragment.
		let fragment = Fragment {
			server: Arc::clone(self),
			artifact: artifact.clone(),
		};

		Ok(fragment)
	}
}
