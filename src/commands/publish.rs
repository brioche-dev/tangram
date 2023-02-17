use crate::{client::Client, os, package, Cli};
use anyhow::{Context, Result};
use std::sync::Arc;

/// Publish a package.
#[derive(clap::Args)]
pub struct Args {
	package: Option<os::PathBuf>,
}

impl Cli {
	pub async fn command_publish(self: &Arc<Self>, args: Args) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().context("Failed to determine the current directory.")?;
		if let Some(path_arg) = args.package {
			path.push(path_arg);
		}

		// Check in the package.
		let package::checkin::Output { package_hash, .. } = self.check_in_package(&path).await?;

		// Create a client.
		let client = Client::new(
			self.api_client.url.clone(),
			None,
			Arc::clone(&self.socket_semaphore),
		);

		// Push the package to the registry.
		self.push(&client, package_hash)
			.await
			.context("Failed to push the package.")?;

		// Publish the package.
		self.api_client
			.publish_package(package_hash)
			.await
			.context("Failed to publish the package.")?;

		Ok(())
	}
}
