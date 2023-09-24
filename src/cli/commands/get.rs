use crate::{error::Result, return_error, Cli};

/// Get an object.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	pub id: tg::object::Id,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_get(&self, _args: Args) -> Result<()> {
		return_error!("This command is not yet implemented.");
	}
}
