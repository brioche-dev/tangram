use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tangram::client::Client;
use url::Url;

#[derive(Parser)]
pub struct Args {
	#[arg(
		long,
		help = "The URL of the API to publish to. Defaults to https://api.tangram.dev.",
		default_value = "https://api.tangram.dev"
	)]
	url: Url,
	package: Option<PathBuf>,
	#[arg(default_value = "https://api.tangram.dev")]
	registry: Url,
	#[arg(long)]
	locked: bool,
}

pub async fn run(args: Args) -> Result<()> {
	// Create the builder.
	let builder = crate::builder().await?.lock_shared().await?;

	// Create the client.
	let client = Client::new(args.registry, None);

	// Get the path.
	let mut path = std::env::current_dir().context("Failed to determine the current directory.")?;
	if let Some(path_arg) = args.package {
		path.push(path_arg);
	}

	// Perform the checkin.
	let hash = builder.checkin(&path).await?;

	// Push the expression to the registry.
	builder
		.push(hash, &client)
		.await
		.context("Failed to push the expression.")?;

	// Publish the package.
	client
		.publish_package(hash)
		.await
		.context("Failed to publish the package.")?;

	Ok(())
}
