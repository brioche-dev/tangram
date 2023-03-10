use crate::Cli;
use tangram::error::Result;

/// Remove unused artifacts.
#[derive(clap::Args)]
pub struct Args {}

impl Cli {
	pub async fn command_clean(&self, _args: Args) -> Result<()> {
		// Collect the roots.
		let roots = Vec::new();

		// Clean.
		self.tg.clean(roots).await?;

		Ok(())
	}
}
