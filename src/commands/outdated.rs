use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Check for outdated dependencies.")]
pub struct Args {
	path: Option<PathBuf>,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_outdated(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().context("Failed to get the current working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}

		Ok(())
	}
}
