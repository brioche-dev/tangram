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
	/// The url of the Tangram server.
	#[clap(long)]
	pub url: Option<Url>,

	/// The hash of the artifact to pull.
	pub artifact_hash: artifact::Hash,
}

impl Cli {
	pub async fn command_push(&self, args: Args) -> Result<()> {
		// Create a client.
		let client = args.url.map(|url| Client::new(url, None));
		let client = client.as_ref().unwrap_or(self.tg.api_client());

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
