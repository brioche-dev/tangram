use crate::{
	artifact::Artifact, fragment::Fragment, object::Dependency, server::Server, util::path_exists,
};
use anyhow::{Context, Result};
use async_recursion::async_recursion;
use futures::FutureExt;
use std::{path::PathBuf, sync::Arc};

impl Server {
	#[async_recursion]
	#[must_use]
	pub async fn create_fragment(self: &Arc<Self>, artifact: Artifact) -> Result<Fragment> {
		// Check if there is an ongoing checkout.
		if let Some(receiver) = self
			.fragment_checkout_task_receivers
			.lock()
			.await
			.get(&artifact)
		{
			let fragment = receiver.resubscribe().recv().await?;
			return Ok(fragment);
		}

		// Lock on the receivers so that only one checkout per artifact can occur simultaneously.
		let mut receivers = self.fragment_checkout_task_receivers.lock().await;

		// Create the broadcast channel.
		let (sender, receiver) = tokio::sync::broadcast::channel::<Fragment>(1);

		// Create the checkout task.
		let checkout_task = tokio::task::spawn({
			let server = Arc::clone(self);
			async move {
				// Get the path to the fragment.
				let fragment_path = server.path().join("fragments").join(artifact.to_string());

				// Check if there is an existing fragment.
				if path_exists(&fragment_path).await? {
					return Ok(Fragment { artifact });
				}

				// Create the path for dependency callback.
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

				Ok::<_, anyhow::Error>(fragment)
			}
		});

		receivers.insert(artifact, receiver);

		// Drop the lock to allow other tasks to run concurrently.
		drop(receivers);

		// Wait for the task to complete.
		let fragment = checkout_task
			.await
			.unwrap()
			.context("The checkout task returned an error.")?;

		// Lock the receivers to send the result and remove this task.
		let mut receivers = self.fragment_checkout_task_receivers.lock().await;

		// Send the fragment to any receivers.
		sender.send(fragment).unwrap();

		// Remove this task's receiver.
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
