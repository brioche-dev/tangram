use crate::{specifier::Specifier, Cli};
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(about = "Format the files in a package.")]
pub struct Args {
	#[arg(default_value = ".")]
	specifier: Specifier,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub(crate) async fn command_fmt(&self, _args: Args) -> Result<()> {
		Ok(())
	}
}
