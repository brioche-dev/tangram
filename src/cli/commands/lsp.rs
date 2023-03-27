use crate::{error::Result, Cli};
use std::sync::Arc;

/// Run the language server.
#[derive(Debug, clap::Args)]
pub struct Args {}

impl Cli {
	pub async fn command_lsp(&self, _args: Args) -> Result<()> {
		// Create the language server.
		let server = tangram::lsp::Server::new(Arc::clone(&self.tg));

		// Run the language server.
		server.serve().await?;

		Ok(())
	}
}
