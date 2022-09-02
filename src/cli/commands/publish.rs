use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Args {
	// #[clap(
	// 	long,
	// 	help = "The URI of the API to publish to. Defaults to https://api.tangram.dev.",
	// 	default_value = "https://api.tangram.dev"
	// )]
	// url: Url,
	package: Option<PathBuf>,
}

pub async fn run(args: Args) -> Result<()> {
	// Create the client.
	let client = crate::client::new().await?;

	// Get the path.
	let package = if let Some(path) = args.package {
		path
	} else {
		std::env::current_dir().context("Failed to determine the current directory.")?
	};

	// Publish the package.
	let artifact = client.publish_package(&package).await?;

	println!("{artifact}");

	Ok(())
}
