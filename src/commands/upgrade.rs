use crate::Cli;
use anyhow::Result;
use std::os::unix::process::CommandExt;

/// Upgrade tangram to the latest version.
#[derive(clap::Args)]
pub struct Args {}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_upgrade(&self, _args: Args) -> Result<()> {
		Err(std::process::Command::new("sh")
			.args(["-c", "curl https://tangram.dev/install.sh | sh"])
			.exec()
			.into())
	}
}
