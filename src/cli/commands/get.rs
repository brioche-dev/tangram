use crate::{Cli, Error, Result};

/// Get an object.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	pub id: tg::object::Id,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_get(&self, args: Args) -> Result<()> {
		let handle = tg::object::Handle::with_id(args.id);
		let data = handle.data(&self.client).await?;
		let string = serde_json::to_string_pretty(&data).map_err(Error::other)?;
		println!("{string}");
		Ok(())
	}
}
