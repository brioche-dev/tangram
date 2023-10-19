use crate::Cli;
use tangram_client as tg;
use tg::{return_error, Result};

/// Remove a dependency from a package.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(default_value = ".")]
	pub package: tg::package::Specifier,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_remove(&self, _args: Args) -> Result<()> {
		return_error!("This command is not yet implemented.");
	}
}
