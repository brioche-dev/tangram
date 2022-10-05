use crate::Cli;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args {}

impl Cli {
	pub(crate) async fn command_gc(&self, _args: Args) -> Result<()> {
		// Lock the builder.
		let builder = self.builder.lock_exclusive().await?;

		// Collect the roots.
		let roots = Vec::new();

		// Perform the garbage collection.
		builder.garbage_collect(roots).await?;

		Ok(())
	}
}
