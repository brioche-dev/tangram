use crate::{Cli, Result};
use std::path::PathBuf;
use tangram_client as tg;
use tg::package::ROOT_MODULE_FILE_NAME;

/// Format the files in a package.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(default_value = ".")]
	pub path: PathBuf,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_fmt(&self, args: Args) -> Result<()> {
		// Create the language server.
		let server =
			tangram_lsp::Server::new(self.client.as_ref(), tokio::runtime::Handle::current());

		let path = args.path.join(ROOT_MODULE_FILE_NAME);
		let text = tokio::fs::read_to_string(&path).await?;
		let text = server.format(text).await?;
		tokio::fs::write(&path, text).await?;

		Ok(())
	}
}
