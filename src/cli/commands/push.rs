use crate::{
	error::{Result, WrapErr},
	Cli,
};
use url::Url;

/// Push a value.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The URL of the Tangram server.
	#[clap(long)]
	pub url: Option<Url>,

	/// The ID of the value to push.
	pub id: tg::Id,
}

impl Cli {
	pub async fn command_push(&self, args: Args) -> Result<()> {
		// Create a client.
		let client = args.url.map(|url| tg::Client::new(url, None));
		let client = client.as_ref().unwrap_or(self.tg.origin_client());

		// Push.
		// client
		// 	.push(&self.tg, block)
		// 	.await
		// 	.wrap_err("Failed to push the artifact.")?;

		Ok(())
	}
}
