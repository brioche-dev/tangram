use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use tangram_core::hash::Hash;

#[derive(Parser, Debug)]
pub struct Args {
	pub hash: Hash,
}

impl Cli {
	pub async fn command_pull(&self, args: Args) -> Result<()> {
		// Lock the builder.
		let builder = self.builder.lock_shared().await?;

		// Pull.
		builder.pull(args.hash).await.context("Failed to pull.")?;

		Ok(())
	}
}
