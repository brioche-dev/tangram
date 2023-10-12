use crate::{Cli, Result};
use tangram_client as tg;

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
		println!("{handle:?}");
		Ok(())
	}
}
