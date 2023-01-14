use crate::{specifier::Specifier, Cli};
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(about = "Add a dependency to a package.")]
pub struct Args {
	#[arg(default_value = ".")]
	specifier: Specifier,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_add(&self, _args: Args) -> Result<()> {
		Ok(())
	}
}
