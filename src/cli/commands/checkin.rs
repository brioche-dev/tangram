use crate::{
	error::{Result, WrapErr},
	Cli,
};
use std::path::PathBuf;
use tangram::artifact::Artifact;

/// Check in an artifact.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The path to check in.
	pub path: Option<PathBuf>,
}

impl Cli {
	pub async fn command_checkin(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path = std::env::current_dir().wrap_err("Failed to get the working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}

		// Perform the checkin.
		let artifact = Artifact::check_in(&self.tg, &path).await?;

		// Print the ID.
		let id = artifact.block().id();
		println!("{id}");

		Ok(())
	}
}
