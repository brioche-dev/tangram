use crate::{error::Result, return_error, Cli};

/// Run the language server.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {}

impl Cli {
	pub async fn command_lsp(&self, _args: Args) -> Result<()> {
		return_error!("This command is not yet implemented.");

		// // Create the language server.
		// let server = tg::language::Server::new(&self.client);

		// // Run the language server.
		// server.serve().await?;

		// Ok(())
	}
}
