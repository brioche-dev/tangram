use crate::config::Config;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tangram::client::Client;
use url::Url;

#[derive(Parser)]
pub struct Args {
	#[clap(
		long,
		help = "The URL of the API to publish to. Defaults to https://api.tangram.dev.",
		default_value = "https://api.tangram.dev"
	)]
	url: Url,
	package: Option<PathBuf>,
}

pub async fn run(args: Args) -> Result<()> {
	// Read the config.
	let config = Config::read().await.context("Failed to read the config.")?;

	// Create the client.
	let client = Client::new_with_config(config.client)
		.await
		.context("Failed to create the client.")?;

	// Get the path.
	let package = if let Some(path) = args.package {
		path
	} else {
		std::env::current_dir().context("Failed to determine the current directory.")?
	};

	// Publish the package.
	let artifact = client
		.publish_package(&package)
		.await
		.context("Failed to publish the package.")?;

	// Print the artifact.
	println!("{artifact}");

	Ok(())
}
