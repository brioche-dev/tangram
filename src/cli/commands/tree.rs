use crate::{
	error::{return_error, Result},
	Cli,
};
use tangram::package;

/// Print the dependency tree of a package.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(default_value = ".")]
	pub package: package::Specifier,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_tree(&self, _args: Args) -> Result<()> {
		return_error!("This command is not yet implemented.");
	}
}
