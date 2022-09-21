use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args {}

pub async fn run(_args: Args) -> Result<()> {
	tokio::process::Command::new("sh")
		.args(["-c", "curl https://tangram.dev/install.sh | sh"])
		.spawn()
		.unwrap()
		.wait()
		.await
		.unwrap();
	Ok(())
}
