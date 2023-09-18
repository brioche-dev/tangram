use crate::{
	error::{Result, WrapErr},
	Cli,
};
use url::Url;

/// Push a value.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The ID of the value to push.
	pub id: tg::Id,
}

impl Cli {
	pub async fn command_push(&self, args: Args) -> Result<()> {
		client
			.push(&self.tg, args.id)
			.await
			.wrap_err("Failed to push the artifact.")?;
		Ok(())
	}
}
