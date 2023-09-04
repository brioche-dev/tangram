use crate::{error::Result, Cli};

/// Run the language server.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {}

impl Cli {
	pub async fn command_lsp(&self, _args: Args) -> Result<()> {
		// Create the language server.
		let server = tg::language::Server::new(self.tg.clone());

		// Run the language server.
		server.serve().await?;

		Ok(())
	}
}
