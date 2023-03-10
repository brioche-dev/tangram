use crate::Cli;
use tangram::{
	artifact,
	error::{Context, Result},
	os,
};

/// Check out an artifact.
#[derive(clap::Args)]
pub struct Args {
	artifact_hash: artifact::Hash,
	path: Option<os::PathBuf>,
}

impl Cli {
	pub async fn command_checkout(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().context("Failed to determine the current directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		} else {
			path.push(args.artifact_hash.to_string());
		};

		// Perform the checkout.
		self.tg
			.check_out_external(args.artifact_hash, &path)
			.await
			.context("Failed to perform the checkout.")?;

		Ok(())
	}
}
