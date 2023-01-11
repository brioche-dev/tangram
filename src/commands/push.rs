use crate::{artifact::ArtifactHash, client::Client, Cli};
use anyhow::{Context, Result};
use clap::Parser;
use url::Url;

#[derive(Parser, Debug)]
pub struct Args {
	pub artifact_hash: ArtifactHash,
	pub url: Url,
}

impl Cli {
	pub async fn command_push(&self, args: Args) -> Result<()> {
		// Lock the cli.
		let cli = self.lock_shared().await?;

		// Create a client.
		let client = Client::new(args.url, None);

		// Push.
		cli.push(&client, args.artifact_hash)
			.await
			.context("Failed to push.")?;

		Ok(())
	}
}
