use crate::{Cli, Result};

/// Run the language server.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {}

impl Cli {
	pub async fn command_lsp(&self, _args: Args) -> Result<()> {
		// Create the language server.
		let server = tg::lsp::Server::new(self.client.clone());

		// Run the language server.
		server.serve().await?;

		Ok(())
	}
}
