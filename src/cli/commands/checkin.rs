use crate::{
	error::{Result, WrapErr},
	Cli,
};
use tangram::util::fs;

/// Check in an artifact.
#[derive(Debug, clap::Args)]
pub struct Args {
	/// The path to check in.
	pub path: Option<fs::PathBuf>,
}

impl Cli {
	pub async fn command_checkin(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().wrap_err("Failed to determine the current directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}

		// Perform the checkin.
		let artifact_hash = self.tg.check_in(&path).await?;

		println!("{artifact_hash}");

		Ok(())
	}
}
