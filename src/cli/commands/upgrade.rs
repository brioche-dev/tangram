use crate::{error::Result, Cli};

/// Upgrade to the latest version of tangram.
#[derive(clap::Args)]
pub struct Args {}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_upgrade(&self, _args: Args) -> Result<()> {
		Ok(())
	}
}
