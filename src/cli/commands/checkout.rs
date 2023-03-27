use crate::{
	error::{Result, WrapErr},
	Cli,
};
use tangram::{artifact, util::fs};

/// Check out an artifact.
#[derive(clap::Args)]
pub struct Args {
	artifact_hash: artifact::Hash,
	path: Option<fs::PathBuf>,
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

		// Vendor the artifact.
		let artifact_hash = self
			.tg
			.vendor(args.artifact_hash)
			.await
			.wrap_err("Failed to vendor the artifact.")?;

		// Perform the checkout.
		self.tg
			.check_out_external(artifact_hash, &path)
			.await
			.wrap_err("Failed to perform the checkout.")?;

		Ok(())
	}
}
