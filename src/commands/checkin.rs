use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(long_about = "Check in an artifact.")]
pub struct Args {
	path: Option<PathBuf>,
}

impl Cli {
	pub(crate) async fn command_checkin(&self, args: Args) -> Result<()> {
		// Lock the cli.
		let cli = self.lock_shared().await?;

		// Get the path.
		let mut path =
			std::env::current_dir().context("Failed to determine the current directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}

		// Perform the checkin.
		let hash = cli.checkin(&path).await?;

		println!("{hash}");

		Ok(())
	}
}
