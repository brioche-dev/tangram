use crate::{
	error::{Result, WrapErr},
	Cli,
};
use tangram::artifact;
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
		let client = self.tg.create_client(args.url, None);

		// Push.
		self.tg
			.push(&client, args.artifact_hash)
			.await
			.wrap_err("Failed to push.")?;

		Ok(())
	}
}
