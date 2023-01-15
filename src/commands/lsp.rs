use crate::Cli;
use clap::Parser;

#[derive(Parser)]
#[command(about = "Run the language server.")]
pub struct Args {}

impl Cli {
	pub async fn command_lsp(&self, _args: Args) -> anyhow::Result<()> {
		// Run the language server.
		self.run_language_server().await?;

		Ok(())
	}
}
