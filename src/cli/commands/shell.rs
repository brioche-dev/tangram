pub use super::run::Args;
use anyhow::Result;
use std::path::PathBuf;

pub async fn run(mut args: Args) -> Result<()> {
	// Set the default target name to "shell".
	args.target = args.target.or_else(|| Some("shell".to_owned()));

	// Set the default executable path to "bin/shell".
	args.executable_path = args
		.executable_path
		.or_else(|| Some(PathBuf::from("bin/shell")));

	// Run!
	super::run::run(args).await?;

	Ok(())
}
