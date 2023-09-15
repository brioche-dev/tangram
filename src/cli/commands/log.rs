use crate::{error::Result, return_error, Cli};
use tg::id::Id;

/// Get the log for a build.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The ID of the build to get logs for.
	pub id: Id,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_log(&self, _args: Args) -> Result<()> {
		return_error!("This command is not yet implemented.");
	}
}
