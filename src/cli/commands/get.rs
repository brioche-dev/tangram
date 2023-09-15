use crate::{error::Result, Cli};

/// Get a value.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	pub id: tg::Id,
}

impl Cli {
	pub async fn command_get(&self, args: Args) -> Result<()> {
		let handle = tg::Handle::with_id(args.id);
		let value = handle.value(&self.client).await?;
		println!("{value:?}");
		Ok(())
	}
}
