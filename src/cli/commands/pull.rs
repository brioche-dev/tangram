use crate::{
	error::{Result, WrapErr},
	Cli,
};
use url::Url;

/// Pull a value.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The ID of the value to pull.
	pub id: tg::Id,
}

impl Cli {
	pub async fn command_pull(&self, args: Args) -> Result<()> {
		client
			.pull(&self.tg, args.id)
			.await
			.wrap_err("Failed to push the artifact.")?;
		Ok(())
	}
}
