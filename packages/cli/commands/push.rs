use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use tangram_core::{client::Client, hash::Hash};
use url::Url;

#[derive(Parser, Debug)]
pub struct Args {
	pub hash: Hash,
	pub url: Option<Url>,
}

impl Cli {
	pub async fn command_push(&self, args: Args) -> Result<()> {
		// Lock the builder.
		let builder = self.builder.lock_shared().await?;

		// Get the client.
		let client = if let Some(url) = args.url {
			Client::new(url, None)
		} else {
			self.api_client.client.clone()
		};

		// Push.
		builder
			.push(args.hash, &client)
			.await
			.context("Failed to push.")?;

		Ok(())
	}
}
