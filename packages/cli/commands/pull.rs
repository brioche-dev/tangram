use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use tangram_core::{client::Client, hash::Hash};
use url::Url;

#[derive(Parser, Debug)]
#[command(trailing_var_arg = true)]
pub struct Args {
	pub hash: Hash,
	pub url: Url,
}

impl Cli {
	pub async fn command_pull(&self, args: Args) -> Result<()> {
		// Lock the builder.
		let builder = self.builder.lock_shared().await?;

		// Create the client.
		let client = Client::new(args.url, None);

		builder
			.pull(args.hash, &client)
			.await
			.context("Failed to pull.")?;

		Ok(())
	}
}
