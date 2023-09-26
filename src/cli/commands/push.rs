use crate::{return_error, Cli, Result, WrapErr};
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
		return_error!("This command is not yet implemented.");
	}
}
