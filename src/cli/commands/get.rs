use crate::{error::Result, Cli};
use tokio::io::AsyncWriteExt;

/// Get a value.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	pub id: tg::Id,
}

impl Cli {
	pub async fn command_get(&self, args: Args) -> Result<()> {
		let mut stdout = tokio::io::stdout();
		let value = tg::Any::with_id(args.id);
		let data = value.data(&self.tg).await?;
		stdout.write_all(format!("{data:?}").as_bytes()).await?;
		Ok(())
	}
}
