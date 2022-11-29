use crate::Cli;
use anyhow::Result;
use clap::Parser;
use tangram_core::specifier::Specifier;

#[derive(Parser)]
pub struct Args {
	#[arg(default_value = ".")]
	specifier: Specifier,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub(crate) async fn command_add(&self, _args: Args) -> Result<()> {
		Ok(())
	}
}
