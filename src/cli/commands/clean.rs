use crate::{error::Result, Cli};

/// Remove unused objects.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {}

impl Cli {
	pub async fn command_clean(&self, _args: Args) -> Result<()> {
		// Clean.
		self.client.clean().await?;

		Ok(())
	}
}
