use crate::{Cli, Result, WrapErr};
use std::path::PathBuf;

/// Check out an artifact.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The ID of the artifact to check out.
	pub id: tg::artifact::Id,

	/// The path to check out the artifact to.
	pub path: Option<PathBuf>,
}

impl Cli {
	pub async fn command_checkout(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path = std::env::current_dir().wrap_err("Failed to get the working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		} else {
			path.push(args.id.to_string());
		};

		// Check out the artifact.
		tg::Artifact::with_id(args.id)
			.check_out(self.client.as_ref(), &path)
			.await
			.wrap_err("Failed to check out the artifact.")?;

		Ok(())
	}
}
