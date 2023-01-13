use crate::{artifact::ArtifactHash, client::Client, Cli};
use anyhow::{Context, Result};
use clap::Parser;
use url::Url;

#[derive(Parser, Debug)]
#[command(about = "Push an artifact.")]
pub struct Args {
	pub artifact_hash: ArtifactHash,
	pub url: Url,
}

impl Cli {
	pub async fn command_push(&self, args: Args) -> Result<()> {
		// Create a client.
		let client = Client::new(args.url, None);

		// Push.
		self.push(&client, args.artifact_hash)
			.await
			.context("Failed to push.")?;

		Ok(())
	}
}
