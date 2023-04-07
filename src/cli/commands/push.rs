use crate::{
	error::{Result, WrapErr},
	Cli,
};
use tangram::{
	artifact::{self, Artifact},
	client::Client,
};
use url::Url;

/// Push an artifact.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	pub artifact_hash: artifact::Hash,

	pub url: Url,
}

impl Cli {
	pub async fn command_push(&self, args: Args) -> Result<()> {
		// Create a client.
		let client = Client::new(args.url, None);

		// Get the artifact.
		let artifact = Artifact::get(&self.tg, args.artifact_hash).await?;

		// Push the artifact.
		client
			.push(&self.tg, &artifact)
			.await
			.wrap_err("Failed to push the artifact.")?;

		Ok(())
	}
}
