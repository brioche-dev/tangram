use crate::Cli;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args {}

impl Cli {
	pub(crate) async fn command_upgrade(&self, _args: Args) -> Result<()> {
		tokio::process::Command::new("sh")
			.args(["-c", "curl https://tangram.dev/install.sh | sh"])
			.spawn()
			.unwrap()
			.wait()
			.await
			.unwrap();
		Ok(())
	}
}
