use crate::{return_error, Cli, Result};

/// Format the files in a package.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(default_value = ".")]
	pub package: tg::package::Specifier,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_fmt(&self, _args: Args) -> Result<()> {
		return_error!("This command is not yet implemented.");
	}
}
