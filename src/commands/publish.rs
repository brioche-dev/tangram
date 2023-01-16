use crate::{client::Client, Cli};
use anyhow::{Context, Result};
use clap::Parser;
use std::{path::PathBuf, sync::Arc};

#[derive(Parser)]
pub struct Args {
	#[arg(long)]
	locked: bool,
	package: Option<PathBuf>,
}

impl Cli {
	pub async fn command_publish(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().context("Failed to determine the current directory.")?;
		if let Some(path_arg) = args.package {
			path.push(path_arg);
		}

		// Check in the package.
		let package_hash = self.checkin_package(&path, args.locked).await?;

		// Get the package.
		let package = self.get_package_local(package_hash)?;

		// Create a client.
		let client = Client::new(
			self.inner.api_client.url.clone(),
			None,
			Arc::clone(&self.inner.socket_semaphore),
		);

		// Push the package source to the registry.
		self.push(&client, package.source)
			.await
			.context("Failed to push the package.")?;

		// Publish the package.
		self.inner
			.api_client
			.publish_package(package.source)
			.await
			.context("Failed to publish the package.")?;

		Ok(())
	}
}
