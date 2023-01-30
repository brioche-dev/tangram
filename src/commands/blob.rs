use crate::{blob::BlobHash, Cli};
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(about = "Manage blobs.")]
pub struct Args {
	#[command(subcommand)]
	command: Command,
}

#[derive(Parser)]
pub enum Command {
	Get(GetArgs),
}

#[derive(Parser, Debug)]
#[command(about = "Get a blob.")]
pub struct GetArgs {
	blob_hash: BlobHash,
}

impl Cli {
	pub async fn command_blob(&self, args: Args) -> Result<()> {
		// Run the subcommand.
		match args.command {
			Command::Get(args) => self.command_blob_get(args),
		}
		.await?;
		Ok(())
	}

	async fn command_blob_get(&self, args: GetArgs) -> Result<()> {
		// Open stdout.
		let mut stdout = tokio::io::stdout();

		// Copy the blob.
		self.copy_blob_to_writer(args.blob_hash, &mut stdout).await?;

		Ok(())
	}
}
