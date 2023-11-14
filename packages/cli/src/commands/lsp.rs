use std::path::PathBuf;

use crate::Cli;
use tangram_client::package::Builder;
use tangram_error::{Result, WrapErr};
use tokio::io::AsyncReadExt;

/// Run the language server.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(long)]
	pub workspace_root: Option<PathBuf>,
}

impl Cli {
	pub async fn command_lsp(&self, args: Args) -> Result<()> {
		let client = self.client().await?;
		let client = client.as_ref();

		// Create the language server.
		let server = tangram_lsp::Server::new(client, tokio::runtime::Handle::current());

		// Run the language server.
		server.serve().await?;

		Ok(())
	}
}
