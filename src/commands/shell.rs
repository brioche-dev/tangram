use crate::{specifier::Specifier, system::System, Cli};
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
	about = "Build a package's shell target and run it.",
	trailing_var_arg = true
)]
pub struct Args {
	#[arg(long)]
	pub executable_path: Option<PathBuf>,
	#[arg(long)]
	pub locked: bool,
	#[arg(long)]
	pub target: Option<String>,
	#[arg(default_value = ".")]
	pub specifier: Specifier,
	pub trailing_args: Vec<String>,
	#[arg(long)]
	pub system: Option<System>,
}

impl Cli {
	pub async fn command_shell(&self, mut args: Args) -> Result<()> {
		// Set the default target name to "shell".
		args.target = args.target.or_else(|| Some("shell".to_owned()));

		// Set the default executable path to "bin/shell".
		args.executable_path = args
			.executable_path
			.or_else(|| Some(PathBuf::from("bin/shell")));

		// Create the run args.
		let args = super::run::Args {
			executable_path: args.executable_path,
			locked: args.locked,
			target: args.target,
			specifier: args.specifier,
			trailing_args: args.trailing_args,
			system: args.system,
		};

		// Run!
		self.command_run(args).await?;

		Ok(())
	}
}
