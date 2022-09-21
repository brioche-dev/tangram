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
	#[clap(long, takes_value = false)]
	locked: bool,
}

pub async fn run(args: Args) -> Result<()> {
	// Read the config.
	let config = Config::read().await.context("Failed to read the config.")?;

	// Create the client.
	let client = Client::new_with_config(config.client)
		.await
		.context("Failed to create the client.")?;

	// Get the path.
	let mut path = std::env::current_dir().context("Failed to determine the current directory.")?;
	if let Some(path_arg) = args.package {
		path.push(path_arg);
	}

	// Publish the package.
	let artifact = client
		.publish_package(&path, args.locked)
		.await
		.context("Failed to publish the package.")?;

	// Print the artifact.
	println!("{artifact}");

	Ok(())
}
