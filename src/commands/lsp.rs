use crate::{compiler::Compiler, lsp::LanguageServer, Cli};
use clap::Parser;

#[derive(Parser)]
pub struct Args {}

impl Cli {
	pub async fn command_lsp(&self, _args: Args) -> anyhow::Result<()> {
		// Create the compiler.
		let compiler = Compiler::new(self.clone());

		// Create the language server.
		let language_server = LanguageServer::new(compiler);

		// Serve!
		language_server.serve().await?;

		Ok(())
	}
}
