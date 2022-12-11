pub use super::run::Args;
use crate::Cli;
use anyhow::Result;
use std::path::PathBuf;

impl Cli {
	pub(crate) async fn command_shell(&self, mut args: Args) -> Result<()> {
		// Set the default target name to "shell".
		args.target = args.target.or_else(|| Some("shell".to_owned()));

		// Set the default executable path to "bin/shell".
		args.executable_path = args
			.executable_path
			.or_else(|| Some(PathBuf::from("bin/shell")));

		// Run!
		self.command_run(args).await?;

		Ok(())
	}
}
