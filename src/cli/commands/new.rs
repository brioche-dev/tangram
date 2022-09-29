pub use super::init::Args;
use anyhow::{Context, Result};

pub async fn run(args: Args) -> Result<()> {
	// Get the path.
	let mut path =
		std::env::current_dir().context("Failed to get the current working directory.")?;
	if let Some(path_arg) = &args.path {
		path.push(path_arg);
	}

	// Create a directory at the path.
	tokio::fs::create_dir_all(&path)
		.await
		.with_context(|| format!(r#"Failed to create the directory at "{}"."#, path.display()))?;

	// Run init.
	super::init::run(args).await?;

	Ok(())
}
