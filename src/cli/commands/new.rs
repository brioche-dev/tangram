use crate::{Cli, Result, WrapErr};
use std::path::PathBuf;

/// Create a new package.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	pub path: Option<PathBuf>,
}

impl Cli {
	pub async fn command_new(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path = std::env::current_dir().wrap_err("Failed to get the working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}

		// Create a directory at the path.
		tokio::fs::create_dir_all(&path).await.wrap_err_with(|| {
			let path = path.display();
			format!(r#"Failed to create the directory at "{path}"."#)
		})?;

		// Init.
		self.command_init(super::init::Args { path: args.path })
			.await?;

		Ok(())
	}
}
