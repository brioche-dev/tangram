use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tangram_core::hash::Hash;

#[derive(Parser)]
pub struct Args {
	artifact: Hash,
	path: Option<PathBuf>,
}

impl Cli {
	pub(crate) async fn command_checkout(&self, args: Args) -> Result<()> {
		// Lock the builder.
		let builder = self.builder.lock_shared().await?;

		// Get the path.
		let mut path =
			std::env::current_dir().context("Failed to determine the current directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		} else {
			path.push(args.artifact.to_string());
		};

		// Perform the checkout.
		builder
			.checkout(args.artifact, &path, None)
			.await
			.context("Failed to perform the checkout.")?;

		Ok(())
	}
}
