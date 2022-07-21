use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tangram::hash::Hash;

#[derive(Parser)]
pub struct Args {
	hash: Hash,
	path: Option<PathBuf>,
}

pub async fn run(_args: Args) -> Result<()> {
	Ok(())
}
