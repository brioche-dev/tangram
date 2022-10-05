use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use tangram_core::{client::Client, hash::Hash};
use url::Url;

#[derive(Parser, Debug)]
pub struct Args {
	pub url: Option<Url>,
	pub hash: Hash,
}

impl Cli {
	pub async fn command_pull(&self, args: Args) -> Result<()> {
		// Lock the builder.
		let builder = self.builder.lock_shared().await?;

		// Get the client.
		let client = if let Some(url) = args.url {
			Client::new(url, None)
		} else {
			self.api_client.client.clone()
		};

		builder
			.pull(args.hash, &client)
			.await
			.context("Failed to pull.")?;

		Ok(())
	}
}
