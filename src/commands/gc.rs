use crate::Cli;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(about = "Perform garbage collection.")]
pub struct Args {}

impl Cli {
	pub(crate) async fn command_gc(&self, _args: Args) -> Result<()> {
		// Lock the cli.
		let mut cli = self.lock_exclusive().await?;

		// Collect the roots.
		let roots = Vec::new();

		// Perform the garbage collection.
		cli.garbage_collect(roots).await?;

		Ok(())
	}
}
