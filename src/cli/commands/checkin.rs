use crate::{
	error::{Result, WrapErr},
	Cli,
};
use tangram::{artifact::Artifact, util::fs};

/// Check in an artifact.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The path to check in.
	pub path: Option<fs::PathBuf>,
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

		// Print the artifact hash.
		let hash = artifact.hash();
		println!("{hash}");

		Ok(())
	}
}
