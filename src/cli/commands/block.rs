use crate::{error::Result, Cli};
use tangram::{block::Block, id::Id};
use tokio::io::AsyncWriteExt;

/// Manage blocks.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[command(subcommand)]
	pub command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
	Bytes(BytesArgs),
	Children(BytesArgs),
	Data(BytesArgs),
}

#[derive(Debug, clap::Args)]
pub struct BytesArgs {
	/// The ID of the block.
	pub id: Id,
}

impl Cli {
	pub async fn command_block(&self, args: Args) -> Result<()> {
		match args.command {
			Command::Bytes(args) => self.command_block_bytes(args).await,
			Command::Children(args) => self.command_block_children(args).await,
			Command::Data(args) => self.command_block_data(args).await,
		}
	}

	async fn command_block_bytes(&self, args: BytesArgs) -> Result<()> {
		let mut stdout = tokio::io::stdout();
		let block = Block::with_id(args.id);
		let bytes = block.bytes(&self.tg).await?;
		stdout.write_all(&bytes).await?;
		Ok(())
	}

	async fn command_block_children(&self, args: BytesArgs) -> Result<()> {
		let mut stdout = tokio::io::stdout();
		let block = Block::with_id(args.id);
		let children = block.children(&self.tg).await?;
		for child in children {
			stdout.write_all(child.to_string().as_bytes()).await?;
			stdout.write_all("\n".as_bytes()).await?;
		}
		Ok(())
	}

	async fn command_block_data(&self, args: BytesArgs) -> Result<()> {
		let mut stdout = tokio::io::stdout();
		let block = Block::with_id(args.id);
		let bytes = block.data(&self.tg).await?;
		stdout.write_all(&bytes).await?;
		Ok(())
	}
}
