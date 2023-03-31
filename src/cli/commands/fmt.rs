use crate::{error::Result, Cli};
use tangram::package;

/// Format the files in a package.
#[derive(Debug, clap::Args)]
pub struct Args {
	#[arg(default_value = ".")]
	pub package: package::Specifier,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_fmt(&self, _args: Args) -> Result<()> {
		Ok(())
	}
}
