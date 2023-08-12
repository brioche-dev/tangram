use crate::{error::Result, Cli};
use tangram::{blob::Blob, block::Block, id::Id};
use tokio::io::AsyncWriteExt;

/// Manage blobs.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[command(subcommand)]
	pub command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
	Get(GetArgs),
}

#[derive(Debug, clap::Args)]
pub struct GetArgs {
	pub id: Id,
}

impl Cli {
	pub async fn command_blob(&self, args: Args) -> Result<()> {
		match args.command {
			Command::Get(args) => self.command_block_get(args).await,
		}
	}

	async fn command_block_get(&self, args: GetArgs) -> Result<()> {
		let mut stdout = tokio::io::stdout();
		let block = Block::with_id(args.id);
		let blob = Blob::with_block(&self.tg, block).await?;
		let bytes = blob.bytes(&self.tg).await?;
		stdout.write_all(&bytes).await?;
		Ok(())
	}
}
