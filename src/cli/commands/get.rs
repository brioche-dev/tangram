use crate::{error::Result, Cli};

/// Get an object.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	pub id: tg::object::Id,
}

impl Cli {
	pub async fn command_get(&self, args: Args) -> Result<()> {
		// let handle = tg::object::Handle::with_id(args.id);
		// let object = handle.object(&self.client).await?;
		// println!("{object:?}");
		Ok(())
	}
}
