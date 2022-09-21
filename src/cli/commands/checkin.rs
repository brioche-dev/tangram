use crate::config::Config;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tangram::client::Client;

#[derive(Parser)]
pub struct Args {
	path: Option<PathBuf>,
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
	if let Some(path_arg) = args.path {
		path.push(path_arg);
	}

	// Perform the checkin.
	let artifact = client.checkin(&path).await?;

	println!("{artifact}");

	Ok(())
}
