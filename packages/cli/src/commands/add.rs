use crate::Cli;
use tangram_error::{return_error, Result};

/// Add a dependency to a package.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(default_value = ".")]
	pub package: tangram_package::Specifier,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_add(&self, _args: Args) -> Result<()> {
		return_error!("This command is not yet implemented.");
	}
}
