pub use super::init::Args;
use crate::Cli;
use anyhow::{Context, Result};

impl Cli {
	pub async fn command_new(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().context("Failed to get the current working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}

		// Create a directory at the path.
		tokio::fs::create_dir_all(&path).await.with_context(|| {
			format!(r#"Failed to create the directory at "{}"."#, path.display())
		})?;

		// Init.
		self.command_init(args).await?;

		Ok(())
	}
}
