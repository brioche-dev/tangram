use crate::Cli;
use clap::Parser;
use tangram_core::js;

#[derive(Parser)]
pub struct Args {}

impl Cli {
	pub async fn command_lsp(&self, _args: Args) -> anyhow::Result<()> {
		// Create the compiler.
		let compiler = js::Compiler::new(self.builder.clone());

		// Create the language server and serve it over stdin/stdout.
		let stdin = tokio::io::stdin();
		let stdout = tokio::io::stdout();
		let (service, socket) =
			tower_lsp::LspService::new(|client| js::LanguageServer::new(client, compiler));
		tower_lsp::Server::new(stdin, stdout, socket)
			.serve(service)
			.await;

		Ok(())
	}
}
