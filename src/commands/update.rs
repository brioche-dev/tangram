use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Update a package's dependencies.")]
pub struct Args {
	path: Option<PathBuf>,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub(crate) async fn command_update(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().context("Failed to get the current working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}

		// Generate the lockfile.
		self.generate_lockfile(&path, true).await?;

		Ok(())
	}
}
