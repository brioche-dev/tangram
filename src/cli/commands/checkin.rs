use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Args {
	path: Option<PathBuf>,
}

pub async fn run(args: Args) -> Result<()> {
	// Create the client.
	let client = crate::client::new().await?;

	// Get the path.
	let mut path = std::env::current_dir().context("Failed to determine the current directory.")?;
	if let Some(path_arg) = &args.path {
		path.push(path_arg);
	}

	// Perform the checkin.
	let artifact = client.checkin(&path).await?;

	println!("{artifact}");

	Ok(())
}
