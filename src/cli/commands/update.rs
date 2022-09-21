use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args {}

#[allow(clippy::unused_async)]
pub async fn run(_args: Args) -> Result<()> {
	todo!()
}
