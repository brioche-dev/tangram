use crate::{error::Result, Cli};
use tangram::{artifact::Artifact, id::Id};
use tokio::io::AsyncWriteExt;

/// Manage artifacts.
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
	pub async fn command_artifact(&self, args: Args) -> Result<()> {
		match args.command {
			Command::Get(args) => self.command_artifact_get(args).await,
		}
	}

	async fn command_artifact_get(&self, args: GetArgs) -> Result<()> {
		let mut stdout = tokio::io::stdout();
		let artifact = Artifact::with_id(id).await?;
		stdout.write_all(format!("{artifact:?}").as_bytes()).await?;
		Ok(())
	}
}
