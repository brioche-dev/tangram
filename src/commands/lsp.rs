use crate::Cli;

/// Run the language server.
#[derive(clap::Args)]
pub struct Args {}

impl Cli {
	pub async fn command_lsp(&self, _args: Args) -> anyhow::Result<()> {
		// Run the language server.
		self.tg.run_lsp().await?;

		Ok(())
	}
}
