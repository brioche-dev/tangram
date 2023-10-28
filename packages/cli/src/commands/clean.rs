use crate::Cli;
use tangram_client as tg;
use tg::Result;

/// Remove unused objects.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {}

impl Cli {
	pub async fn command_clean(&self, _args: Args) -> Result<()> {
		let client = self.client().await?;
		let client = client.as_ref();

		// Clean.
		client.clean().await?;

		Ok(())
	}
}
