use crate::{hash::Hash, Cli};
use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
	pub hash: Hash,
}

impl Cli {
	pub async fn command_pull(&self, args: Args) -> Result<()> {
		// Lock the cli.
		let cli = self.lock_shared().await?;

		// Pull.
		cli.pull(args.hash).await.context("Failed to pull.")?;

		Ok(())
	}
}
