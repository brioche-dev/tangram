use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args {}

pub async fn run(_args: Args) -> Result<()> {
	let client = crate::client::create().await?;
	client.garbage_collect().await?;
	Ok(())
}
