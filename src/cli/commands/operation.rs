use crate::{error::Result, Cli};
use tangram::{block::Block, id::Id, operation::Operation};
use tokio::io::AsyncWriteExt;

/// Manage operations.
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
	pub async fn command_operation(&self, args: Args) -> Result<()> {
		match args.command {
			Command::Get(args) => self.command_operation_get(args).await,
		}
	}

	async fn command_operation_get(&self, args: GetArgs) -> Result<()> {
		let mut stdout = tokio::io::stdout();
		let block = Block::with_id(args.id);
		let operation = Operation::with_block(&self.tg, block).await?;
		let json = serde_json::to_string_pretty(&operation.to_data()).unwrap();
		stdout.write_all(json.as_bytes()).await?;
		Ok(())
	}
}
