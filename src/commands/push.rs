use crate::{hash::Hash, Cli};
use anyhow::{Context, Result};
use clap::Parser;
use url::Url;

#[derive(Parser, Debug)]
pub struct Args {
	pub hash: Hash,
	pub url: Option<Url>,
}

impl Cli {
	pub async fn command_push(&self, args: Args) -> Result<()> {
		// Lock the cli.
		let cli = self.lock_shared().await?;

		// Push.
		cli.push(args.hash).await.context("Failed to push.")?;

		Ok(())
	}
}
