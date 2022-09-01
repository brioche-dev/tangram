use crate::{artifact::Artifact, object::Dependency, server::Server, util::path_exists};
use anyhow::{Context, Result};
use async_recursion::async_recursion;
use futures::FutureExt;
use std::{path::PathBuf, sync::Arc};
use tracing::Instrument;

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
		// Check if there is an ongoing checkout.
		tracing::info!("Creating fragment for {artifact}");

		// Get the path to the fragment.
		let fragment_path = self.path().join("fragments").join(artifact.to_string());

		// Check if there is an existing fragment.
		if path_exists(&fragment_path).await? {
			tracing::info!("Fragment already materialized.");
			return Ok(Fragment { artifact });
		}

		let receivers = self.fragment_checkout_task_receivers.lock().await;
		let receiver = receivers
			.get(&artifact)
			.map(tokio::sync::broadcast::Receiver::resubscribe);
		drop(receivers);
		if let Some(mut receiver) = receiver {
			tracing::info!("There is an ongoing checkout.");
			let fragment = receiver.recv().await?;
			tracing::info!("Ongoing checkout complete.");
			return Ok(fragment);
		}

		// Create the broadcast channel.
		let (sender, receiver) = tokio::sync::broadcast::channel::<Fragment>(1);

		// Lock on the receivers so that only one checkout per artifact can occur simultaneously.
		tracing::info!("Attempting to lock receivers for insertion.");
		let mut receivers = self.fragment_checkout_task_receivers.lock().await;
		tracing::info!("Locked receivers.");
		// Add the receiver.
		receivers.insert(artifact, receiver);
		// Drop the lock to allow other tasks to run concurrently.
		drop(receivers);
		tracing::info!("Unlocked receivers.");

		// Create the checkout task.
		let checkout_task = {
			let server = Arc::clone(self);
			async move {
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
				tracing::info!("Performing the checkout.");
				server
					.checkout(artifact, &fragment_path, Some(&path_for_dependency))
					.await?;
				tracing::info!("Checkout complete.");

				// Create the fragment.
				let fragment = Fragment { artifact };

				Ok::<_, anyhow::Error>(fragment)
			}
		};

		// Wait for the task to complete.
		let checkout_handle = tokio::task::spawn(checkout_task);
		let fragment = checkout_handle
			.await
			.unwrap()
			.context("The checkout task returned an error.")?;

		// Send the fragment to any receivers.
		sender.send(fragment).unwrap();

		// Lock the receivers to send the fragment and remove the receiver.
		tracing::info!("Attempting to lock receivers for removal.");
		let mut receivers = self.fragment_checkout_task_receivers.lock().await;
		tracing::info!("Locked receivers.");
		// Remove this task's receiver.
		receivers.remove(&artifact);
		drop(receivers);
		tracing::info!("Unlocked receivers.");

		Ok(fragment)
	}

	#[must_use]
	pub fn fragment_path(self: &Arc<Self>, fragment: &Fragment) -> PathBuf {
		self.path()
			.join("fragments")
			.join(fragment.artifact().to_string())
	}
}
