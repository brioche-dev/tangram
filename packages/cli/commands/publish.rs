use crate::Cli;
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
		// Lock the builder.
		let builder = self.builder.lock_shared().await?;

		// Get the path.
		let mut path =
			std::env::current_dir().context("Failed to determine the current directory.")?;
		if let Some(path_arg) = args.package {
			path.push(path_arg);
		}

		// Check in the package.
		let package_hash = builder
			.checkin_package(&self.api_client, &path, args.locked)
			.await?;

		// Push the package to the registry.
		builder
			.push(package_hash)
			.await
			.context("Failed to push the expression.")?;

		// Publish the package.
		self.api_client
			.publish_package(package_hash)
			.await
			.context("Failed to publish the package.")?;

		Ok(())
	}
}
