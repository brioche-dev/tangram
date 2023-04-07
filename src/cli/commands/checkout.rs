use crate::{
	error::{Result, WrapErr},
	Cli,
};
use tangram::{
	artifact::{self, Artifact},
	util::fs,
};

/// Check out an artifact.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The hash of the artifact to check out.
	pub artifact_hash: artifact::Hash,

	/// The path to check out the artifact to.
	pub path: Option<fs::PathBuf>,
}

impl Cli {
	pub async fn command_checkout(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path = std::env::current_dir().wrap_err("Failed to get the working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		} else {
			path.push(args.artifact_hash.to_string());
		};

		// Get the artifact.
		let artifact = Artifact::get(&self.tg, args.artifact_hash)
			.await
			.wrap_err("Failed to get the artifact.")?;

		// Check out the artifact.
		artifact
			.check_out(&self.tg, &path)
			.await
			.wrap_err("Failed to check out the artifact.")?;

		Ok(())
	}
}
