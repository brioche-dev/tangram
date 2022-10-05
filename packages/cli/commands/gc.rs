use crate::Cli;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args {}

impl Cli {
	pub(crate) async fn command_gc(&self, _args: Args) -> Result<()> {
		// Create the builder.
		let builder = crate::builder().await?;

		// Collect the roots.
		let roots = Vec::new();

		// Perform the garbage collection.
		builder
			.lock_exclusive()
			.await?
			.garbage_collect(roots)
			.await?;

		Ok(())
	}
}
