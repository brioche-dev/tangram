use crate::{error::Result, Cli};

#[derive(clap::Args)]
pub struct Args {}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_exec(&self, _args: Args) -> Result<()> {
		Ok(())
	}
}
