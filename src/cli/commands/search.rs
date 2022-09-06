use crate::config::Config;
use anyhow::{Context, Result};
use clap::Parser;
use tangram::client::Client;

#[derive(Parser)]
pub struct Args {
	name: String,
}

pub async fn run(args: Args) -> Result<()> {
	// Read the config.
	let config = Config::read().await.context("Failed to read the config.")?;

	// Create the client.
	let client = Client::new_with_config(config.client)
		.await
		.context("Failed to create the client.")?;

	// Search for the package with the given name.
	let package_name = args.name;
	let packages = client.search(&package_name).await?;
	println!("{:?}", packages);

	Ok(())
}
