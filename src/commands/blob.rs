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
	pub(crate) async fn command_blob(&self, args: Args) -> Result<()> {
		// Run the subcommand.
		match args.command {
			Command::Get(args) => self.command_blob_get(args),
		}
		.await?;
		Ok(())
	}

	async fn command_blob_get(&self, args: GetArgs) -> Result<()> {
		// Get the blob.
		let mut blob = self.get_blob(args.blob_hash).await?.into_std().await;

		// Open stdout.
		let mut stdout = std::io::stdout();

		// Copy the blob to the path.
		tokio::task::spawn_blocking(move || {
			// Copy the blob to stdout.
			std::io::copy(&mut blob, &mut stdout)?;
			Ok::<_, anyhow::Error>(())
		})
		.await
		.unwrap()?;

		Ok(())
	}
}
