use crate::{
	error::{Result, WrapErr},
	Cli,
};
use tangram::{client::Client, id::Id};
use url::Url;

/// Pull a block.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The URL of the Tangram server.
	#[clap(long)]
	pub url: Option<Url>,

	/// The ID of the block to pull.
	pub id: Id,
}

impl Cli {
	pub async fn command_pull(&self, args: Args) -> Result<()> {
		// Create a client.
		let client = args.url.map(|url| Client::new(url, None));
		let client = client.as_ref().unwrap_or(self.tg.origin_client());

		// // Pull.
		// let block = Block::with_id(args.id);
		// client
		// 	.pull(&self.tg, block)
		// 	.await
		// 	.wrap_err("Failed to pull the block.")?;

		Ok(())
	}
}
