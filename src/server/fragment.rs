use crate::{artifact::Artifact, object::Dependency, server::Server, util::path_exists};
use anyhow::{Context, Result};
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
		// If there is an ongoing checkout, wait for it and return.
		let mut receivers = self.fragment_checkout_task_receivers.lock().await;
		if let Some(receiver) = receivers.get(&artifact) {
			let mut receiver = receiver.resubscribe();
			drop(receivers);
			tracing::info!("Waiting on receiver.");
			let fragment = receiver.recv().await?;
			tracing::info!("Got it!");
			return Ok(fragment);
		}

		// Otherwise, create a new broadcast channel and add its receiver.
		tracing::info!("I am the checker outer.");
		let (sender, receiver) = tokio::sync::broadcast::channel::<Fragment>(1);
		receivers.insert(artifact, receiver);
		drop(receivers);

		// Create the checkout task.
		let checkout_task = tokio::task::spawn({
			let server = Arc::clone(self);
			async move {
				tracing::info!("Performing the checkout!");
				// Get the path to the fragment.
				let fragment_path = server.path().join("fragments").join(artifact.to_string());

				// Check if there is an existing fragment.
				if path_exists(&fragment_path).await? {
					return Ok(Fragment { artifact });
				}

				// Create the callback to create dependency fragments.
				let path_for_dependency = {
					let server = Arc::clone(&server);
					move |dependency: &Dependency| {
						let server = Arc::clone(&server);
						let dependency = dependency.clone();
						async move {
							let dependency_fragment =
								server.create_fragment(dependency.artifact).await?;
							let dependency_fragment_path =
								server.fragment_path(&dependency_fragment);
							Ok(Some(dependency_fragment_path))
						}
						.boxed()
					}
				};

				// Perform the checkout.
				server
					.checkout(artifact, &fragment_path, Some(&path_for_dependency))
					.await?;

				// Create the fragment.
				let fragment = Fragment { artifact };

				tracing::info!("Done!");
				Ok::<_, anyhow::Error>(fragment)
			}
		});

		// Wait for the task to complete.
		let fragment = checkout_task
			.await
			.unwrap()
			.context("The checkout task returned an error.")?;

		// Lock the receivers to send the fragment and remove the receiver.
		let mut receivers = self.fragment_checkout_task_receivers.lock().await;
		sender.send(fragment).unwrap();
		receivers.remove(&artifact);
		drop(receivers);

		Ok(fragment)
	}

	#[must_use]
	pub fn fragment_path(self: &Arc<Self>, fragment: &Fragment) -> PathBuf {
		self.path()
			.join("fragments")
			.join(fragment.artifact().to_string())
	}
}
