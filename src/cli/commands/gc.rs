use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args {}

pub async fn run(_args: Args) -> Result<()> {
	// Create the client.
	let client = crate::client::new().await?;

	// Perform the garbage collection.
	client.garbage_collect().await?;

	Ok(())
}
