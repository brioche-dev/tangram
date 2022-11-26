use crate::Cli;
use clap::Parser;
use tangram_core::js;

#[derive(Parser)]
pub struct Args {}

impl Cli {
	pub async fn command_lsp(&self, _args: Args) -> anyhow::Result<()> {
		// Create the compiler.
		let compiler = js::Compiler::new(self.builder.clone());

		// Create the language server.
		let language_server = js::LanguageServer::new(compiler);

		// Serve!
		language_server.serve().await?;

		Ok(())
	}
}
