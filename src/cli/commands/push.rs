use crate::{
	error::{Result, WrapErr},
	Cli,
};
use tangram::{block::Block, client::Client, id::Id};
use url::Url;

/// Push a block.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The URL of the Tangram server.
	#[clap(long)]
	pub url: Option<Url>,

	/// The ID of the block to push.
	pub id: Id,
}

impl Cli {
	pub async fn command_push(&self, args: Args) -> Result<()> {
		// Create a client.
		let client = args.url.map(|url| Client::new(url, None));
		let client = client.as_ref().unwrap_or(self.tg.api_client());

		// Push.
		let block = Block::with_id(args.id);
		client
			.push(&self.tg, block)
			.await
			.wrap_err("Failed to push the artifact.")?;

		Ok(())
	}
}
