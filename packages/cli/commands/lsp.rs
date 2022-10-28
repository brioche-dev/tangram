use crate::Cli;
use clap::Parser;
use tangram_core::js::{self, lsp::VIRTUAL_TEXT_DOCUMENT_REQUEST};

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
			tower_lsp::LspService::build(|client| js::LanguageServer::new(client, compiler))
				.custom_method(
					VIRTUAL_TEXT_DOCUMENT_REQUEST,
					js::LanguageServer::virtual_text_document,
				)
				.finish();
		tower_lsp::Server::new(stdin, stdout, socket)
			.serve(service)
			.await;

		Ok(())
	}
}
