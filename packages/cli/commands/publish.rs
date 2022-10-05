use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tangram_core::client::Client;
use url::Url;

use crate::api_client;

#[derive(Parser)]
pub struct Args {
	#[arg(long)]
	locked: bool,
	package: Option<PathBuf>,
}

impl Cli {
	pub(crate) async fn command_publish(&self, args: Args) -> Result<()> {
		// // Create the builder.
		// let builder = crate::builder().await?.lock_shared().await?;

		// // Get the path.
		// let mut path = std::env::current_dir().context("Failed to determine the current directory.")?;
		// if let Some(path_arg) = args.package {
		// 	path.push(path_arg);
		// }

		// // Checkin the package.
		// let package_hash = builder.checkin_package(&path, args.locked).await?;

		// // Create the API client.
		// let api_client = api_client().await?;

		// // Push the package to the registry.
		// builder
		// 	.push(package_hash, &api_client)
		// 	.await
		// 	.context("Failed to push the expression.")?;

		// // Publish the package.
		// api_client
		// 	.publish_package(package_hash)
		// 	.await
		// 	.context("Failed to publish the package.")?;

		Ok(())
	}
}
