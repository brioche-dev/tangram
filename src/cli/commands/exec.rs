use crate::{error::Result, Cli};

#[derive(Debug, clap::Args)]
pub struct Args {}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_exec(&self, _args: Args) -> Result<()> {
		Ok(())
	}
}
