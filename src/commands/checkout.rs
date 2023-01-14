use crate::{artifact::ArtifactHash, Cli};
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Check out an artifact.")]
pub struct Args {
	artifact_hash: ArtifactHash,
	path: Option<PathBuf>,
}

impl Cli {
	pub async fn command_checkout(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().context("Failed to determine the current directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		} else {
			path.push(args.artifact_hash.to_string());
		};

		// Perform the checkout.
		self.checkout(args.artifact_hash, &path, None)
			.await
			.context("Failed to perform the checkout.")?;

		Ok(())
	}
}
