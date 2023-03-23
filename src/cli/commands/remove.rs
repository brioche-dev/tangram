use crate::{error::Result, Cli};
use tangram::package;

/// Remove a dependency from a package.
#[derive(clap::Args)]
pub struct Args {
	#[arg(default_value = ".")]
	package_specifier: package::Specifier,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_remove(&self, _args: Args) -> Result<()> {
		Ok(())
	}
}
