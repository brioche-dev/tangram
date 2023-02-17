use crate::Cli;
use std::sync::Arc;

/// Run the language server.
#[derive(clap::Args)]
pub struct Args {}

impl Cli {
	pub async fn command_lsp(self: &Arc<Self>, _args: Args) -> anyhow::Result<()> {
		// Run the language server.
		self.run_language_server().await?;

		Ok(())
	}
}
