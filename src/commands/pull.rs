use crate::{artifact::ArtifactHash, client::Client, Cli};
use anyhow::{Context, Result};
use clap::Parser;
use url::Url;

#[derive(Parser, Debug)]
#[command(about = "Pull an artifact.")]
pub struct Args {
	pub artifact_hash: ArtifactHash,
	pub url: Url,
}

impl Cli {
	pub async fn command_pull(&self, args: Args) -> Result<()> {
		// Create a client.
		let client = Client::new(args.url, None);

		// Pull.
		self.pull(&client, args.artifact_hash)
			.await
			.context("Failed to pull.")?;

		Ok(())
	}
}
