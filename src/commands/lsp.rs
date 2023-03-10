use crate::Cli;
use tangram::error::Result;

/// Run the language server.
#[derive(clap::Args)]
pub struct Args {}

impl Cli {
	pub async fn command_lsp(&self, _args: Args) -> Result<()> {
		// Run the language server.
		self.tg.run_lsp().await?;

		Ok(())
	}
}
