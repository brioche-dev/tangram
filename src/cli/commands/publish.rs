use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
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
	// Create the client.
	let client = crate::client::new().await?;

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
