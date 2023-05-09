use crate::{error::Result, return_error, Cli};
use std::path::PathBuf;

/// Update a package's dependencies.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	pub path: Option<PathBuf>,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_update(&self, _args: Args) -> Result<()> {
		return_error!("This command is not yet implemented.");
	}
}
