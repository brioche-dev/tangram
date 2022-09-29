use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tangram::hash::Hash;

#[derive(Parser)]
pub struct Args {
	artifact: Hash,
	path: Option<PathBuf>,
}

pub async fn run(args: Args) -> Result<()> {
	// Create the client.
	let client = crate::client::new().await?;

	// Get the path.
	let mut path = std::env::current_dir().context("Failed to determine the current directory.")?;
	if let Some(path_arg) = &args.path {
		path.push(path_arg);
	} else {
		path.push(args.artifact.to_string());
	};

	// Perform the checkout.
	client
		.checkout(args.artifact, &path, None)
		.await
		.context("Failed to perform the checkout.")?;

	Ok(())
}
