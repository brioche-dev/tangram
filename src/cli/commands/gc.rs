use crate::config::Config;
use anyhow::{Context, Result};
use clap::Parser;
use tangram::client::Client;

#[derive(Parser)]
pub struct Args {}

pub async fn run(_args: Args) -> Result<()> {
	// Read the config.
	let config = Config::read().await.context("Failed to read the config.")?;

	// Create the client.
	let client = Client::new_with_config(config.client)
		.await
		.context("Failed to create the client.")?;

	// Perform the garbage collection.
	client.garbage_collect().await?;

	Ok(())
}
