use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Args {
	path: Option<PathBuf>,
}

pub async fn run(_args: Args) -> Result<()> {
	Ok(())
}
