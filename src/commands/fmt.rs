use crate::{package, Cli};
use anyhow::Result;

/// Format the files in a package.
#[derive(clap::Args)]
pub struct Args {
	#[arg(default_value = ".")]
	package_specifier: package::Specifier,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_fmt(&self, _args: Args) -> Result<()> {
		Ok(())
	}
}
