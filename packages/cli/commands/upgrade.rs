use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser)]
pub struct Args {}

impl Cli {
	pub(crate) async fn command_upgrade(&self, _args: Args) -> Result<()> {
		tokio::process::Command::new("sh")
			.args(["-c", "curl https://tangram.dev/install.sh | sh"])
			.spawn()
			.context("Failed to spawn the install script.")?
			.wait()
			.await
			.context("Failed to run the install script.")?;
		Ok(())
	}
}
