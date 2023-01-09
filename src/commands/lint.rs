use crate::{specifier::Specifier, Cli};
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(about = "Lint a package.")]
pub struct Args {
	#[arg(default_value = ".")]
	specifier: Specifier,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub(crate) async fn command_lint(&self, _args: Args) -> Result<()> {
		Ok(())
	}
}
