use crate::{artifact, Cli};
use anyhow::{Context, Result};
use url::Url;

/// Push an artifact.
#[derive(clap::Args)]
pub struct Args {
	pub artifact_hash: artifact::Hash,
	pub url: Url,
}

impl Cli {
	pub async fn command_push(&self, args: Args) -> Result<()> {
		// Create a client.
		let client = self.create_client(args.url, None);

		// Push.
		self.push(&client, args.artifact_hash)
			.await
			.context("Failed to push.")?;

		Ok(())
	}
}
