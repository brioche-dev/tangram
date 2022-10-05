use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use tangram_core::{client::Client, hash::Hash};
use url::Url;

#[derive(Parser, Debug)]
#[command(trailing_var_arg = true)]
pub struct Args {
	pub hash: Hash,
	pub url: Option<Url>,
}

impl Cli {
	pub async fn command_push(&self, args: Args) -> Result<()> {
		// Lock the builder.
		let builder = self.builder.lock_shared().await?;

		// Create the client.
		let client = Client::new(args.url.unwrap(), None);

		builder
			.push(args.hash, &client)
			.await
			.context("Failed to push.")?;

		Ok(())
	}
}
