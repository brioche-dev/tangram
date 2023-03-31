use crate::{
	error::{return_error, Result},
	Cli,
};
use tangram::package;

/// Add a dependency to a package.
#[derive(Debug, clap::Args)]
pub struct Args {
	#[arg(default_value = ".")]
	pub package: package::Specifier,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_add(&self, _args: Args) -> Result<()> {
		return_error!("This command is not yet implemented.");
	}
}
