use crate::{client::Client, Cli};
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Args {
	#[arg(long)]
	locked: bool,
	package: Option<PathBuf>,
}

impl Cli {
	pub(crate) async fn command_publish(&self, args: Args) -> Result<()> {
		// Lock the cli.
		let cli = self.lock_shared().await?;

		// Get the path.
		let mut path =
			std::env::current_dir().context("Failed to determine the current directory.")?;
		if let Some(path_arg) = args.package {
			path.push(path_arg);
		}

		// Check in the package.
		let package_hash = cli.checkin_package(&path, args.locked).await?;

		// Get the package.
		let package = cli.get_package_local(package_hash)?;

		// Get the API Url.
		let api_url = self.state.lock_shared().await?.api_url.clone();

		// Create a client.
		let client = Client::new(api_url.clone(), None);

		// Push the package source to the registry.
		cli.push(&client, package.source)
			.await
			.context("Failed to push the package.")?;

		// Publish the package.
		cli.api_client
			.publish_package(package.source)
			.await
			.context("Failed to publish the package.")?;

		Ok(())
	}
}
