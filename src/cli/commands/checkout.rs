use crate::{
	error::{Result, WrapErr},
	Cli,
};
use tangram::{artifact, util::fs};

/// Check out an artifact.
#[derive(Debug, clap::Args)]
pub struct Args {
	/// The hash of the artifact to check out.
	pub artifact_hash: artifact::Hash,

	/// The path to check out the artifact to.
	pub path: Option<fs::PathBuf>,
}

impl Cli {
	pub async fn command_checkout(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().wrap_err("Failed to determine the current directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		} else {
			path.push(args.artifact_hash.to_string());
		};

		// Perform the checkout.
		self.tg
			.check_out_external(args.artifact_hash, &path)
			.await
			.wrap_err("Failed to perform the checkout.")?;

		Ok(())
	}
}
