use crate::Cli;
use anyhow::{Context, Result};
use tangram::{os, package};

/// Publish a package.
#[derive(clap::Args)]
pub struct Args {
	package: Option<os::PathBuf>,
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
		let package::checkin::Output { package_hash, .. } = self.tg.check_in_package(&path).await?;

		// Create a client.
		let client = self.tg.create_client(
			self.tg.api_client().url.clone(),
			self.tg.api_client().token.clone(),
		);

		// Push the package to the registry.
		self.tg
			.push(&client, package_hash)
			.await
			.context("Failed to push the package.")?;

		// Publish the package.
		self.tg
			.api_client()
			.publish_package(package_hash)
			.await
			.context("Failed to publish the package.")?;

		Ok(())
	}
}
