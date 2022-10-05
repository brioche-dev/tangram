use crate::Cli;
use anyhow::Result;
use clap::Parser;
use tangram_core::hash::Hash;

#[derive(Parser)]
pub struct Args {
	#[command(subcommand)]
	subcommand: Subcommand,
}

#[derive(Parser)]
pub enum Subcommand {
	Get(GetArgs),
}

#[derive(Parser, Debug)]
pub struct GetArgs {
	blob_hash: Hash,
}

impl Cli {
	pub(crate) async fn command_blob(&self, args: Args) -> Result<()> {
		// Run the subcommand.
		match args.subcommand {
			Subcommand::Get(args) => self.command_blob_get(args),
		}
		.await?;
		Ok(())
	}

	async fn command_blob_get(&self, args: GetArgs) -> Result<()> {
		// Lock the builder.
		let builder = self.builder.lock_shared().await?;

		// Get the blob.
		let blob_path = builder.get_blob(args.blob_hash).await?;

		// Open the blob file.
		let mut file = tokio::fs::File::open(blob_path).await?;

		// Open stdout.
		let mut stdout = tokio::io::stdout();

		// Copy the blob to stdout.
		tokio::io::copy(&mut file, &mut stdout).await?;

		Ok(())
	}
}
