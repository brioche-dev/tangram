use crate::{
	error::{Result, WrapErr},
	Cli,
};
use tangram::util::fs;

/// Update a package's dependencies.
#[derive(clap::Args)]
pub struct Args {
	path: Option<fs::PathBuf>,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_update(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().wrap_err("Failed to get the current working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}

		// Create the lockfile.
		self.tg.create_lockfile(&path).await?;

		Ok(())
	}
}
