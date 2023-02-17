use crate::{os, Cli};
use anyhow::{Context, Result};

/// Check for outdated dependencies.
#[derive(clap::Args)]
pub struct Args {
	path: Option<os::PathBuf>,
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
