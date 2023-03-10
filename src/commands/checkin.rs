use crate::Cli;
use tangram::{
	error::{Context, Result},
	os,
};

/// Check in an artifact.
#[derive(clap::Args)]
pub struct Args {
	path: Option<os::PathBuf>,
}

impl Cli {
	pub async fn command_checkin(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().context("Failed to determine the current directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}

		// Perform the checkin.
		let artifact_hash = self.tg.check_in(&path).await?;

		println!("{artifact_hash}");

		Ok(())
	}
}
