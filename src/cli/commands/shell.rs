use super::{PackageArgs, RunArgs};
use crate::{error::Result, Cli};
use tangram::package;

/// Build a package's "shell" export and run it.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
#[command(trailing_var_arg = true)]
pub struct Args {
	/// The package to build.
	#[arg(short, long, default_value = ".")]
	pub package: package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,

	#[command(flatten)]
	pub run_args: RunArgs,

	/// Arguments to pass to the executable.
	pub trailing_args: Vec<String>,
}

impl Cli {
	pub async fn command_shell(&self, args: Args) -> Result<()> {
		// Create the run args.
		let args = super::run::Args {
			package: args.package,
			package_args: args.package_args,
			run_args: args.run_args,
			target: "shell".to_owned(),
			trailing_args: args.trailing_args,
		};

		// Run!
		self.command_run(args).await?;

		Ok(())
	}
}
