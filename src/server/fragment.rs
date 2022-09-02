use crate::{artifact::Artifact, object::Dependency, server::Server, util::path_exists};
use anyhow::Result;
use async_recursion::async_recursion;
use futures::FutureExt;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone, Copy, Debug)]
pub struct Fragment {
	pub(crate) artifact: Artifact,
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
		// Acquire a lock to checkout a fragment for this artifact.
		let mutex = Arc::clone(
			self.fragment_checkout_mutexes
				.write()
				.unwrap()
				.entry(artifact)
				.or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))),
		);
		let lock = mutex.lock().await;

		// Check if there is an existing fragment and check one out if necessary.
		let fragment_path = self.path().join("fragments").join(artifact.to_string());
		let fragment = if path_exists(&fragment_path).await? {
			Fragment { artifact }
		} else {
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
			self.checkout(artifact, &fragment_path, Some(&path_for_dependency))
				.await?;

			Fragment { artifact }
		};

		// Drop the lock.
		drop(lock);

		// Remove the lock if it is no longer in use.
		let mut mutexes = self.fragment_checkout_mutexes.write().unwrap();
		if let Some(mutex) = mutexes.get(&artifact) {
			if mutex.try_lock().is_ok() {
				mutexes.remove(&artifact);
			}
		}
		drop(mutexes);

		Ok(fragment)
	}

	#[must_use]
	pub fn fragment_path(self: &Arc<Self>, fragment: &Fragment) -> PathBuf {
		self.path()
			.join("fragments")
			.join(fragment.artifact().to_string())
	}
}
