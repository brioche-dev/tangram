use crate::{error::Result, Cli};
use tangram::package;

/// Add a dependency to a package.
#[derive(Debug, clap::Args)]
pub struct Args {
	#[arg(default_value = ".")]
	package_specifier: package::Specifier,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_add(&self, _args: Args) -> Result<()> {
		Ok(())
	}
}
