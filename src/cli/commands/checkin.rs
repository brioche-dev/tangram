use crate::{
	error::{Result, WrapErr},
	Cli,
};
use std::path::PathBuf;

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
		let artifact = tg::Artifact::check_in(&self.tg, &path).await?;

		// Store the artifact.
		artifact.store(&self.tg).await?;

		// Print the ID.
		let id = artifact.id(&self.tg).await?;
		println!("{id}");

		Ok(())
	}
}
