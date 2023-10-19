use crate::Cli;
use tangram_client as tg;
use tg::Result;

/// Run the language server.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {}

impl Cli {
	pub async fn command_lsp(&self, _args: Args) -> Result<()> {
		// Create the language server.
		let server = tangram_lsp::Server::new(
			self.client.downgrade_box(),
			tokio::runtime::Handle::current(),
		);

		// Run the language server.
		server.serve().await?;

		Ok(())
	}
}
