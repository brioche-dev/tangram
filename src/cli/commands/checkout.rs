use crate::config::Config;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tangram::{client::Client, expression::Artifact, hash::Hash};

#[derive(Parser)]
pub struct Args {
	artifact: Hash,
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
	let path = if let Some(path) = args.path {
		path
	} else {
		std::env::current_dir()
			.context("Failed to determine the current directory.")?
			.join(args.artifact.to_string())
	};

	// Perform the checkout.
	let artifact = Artifact {
		hash: args.artifact,
	};
	client
		.checkout(artifact, &path, None)
		.await
		.context("Failed to perform the checkout.")?;

	Ok(())
}
