use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tangram::artifact::Artifact;

#[derive(Parser)]
pub struct Args {
	artifact: Artifact,
	path: Option<PathBuf>,
}

pub async fn run(args: Args) -> Result<()> {
	// Create the client.
	let client = crate::client::new().await?;

	// Get the path.
	let path = if let Some(path) = args.path {
		path
	} else {
		std::env::current_dir()
			.context("Failed to determine the current directory.")?
			.join(args.artifact.to_string())
	};

	// Perform the checkout.
	client.checkout(args.artifact, &path, None).await?;

	Ok(())
}
