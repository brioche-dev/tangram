use super::{PackageArgs, RunArgs};
use crate::Cli;
use tangram_client as tg;
use tangram_error::Result;

/// Build a package's "shell" export and run it.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
#[command(trailing_var_arg = true)]
pub struct Args {
	/// The package to build.
	#[arg(short, long, default_value = ".")]
	pub package: tangram_lsp::package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,

	/// The retry strategy to use.
	#[arg(long, default_value_t)]
	pub retry: tg::build::Retry,

	#[command(flatten)]
	pub run_args: RunArgs,

	/// Arguments to pass to the executable.
	pub trailing_args: Vec<String>,
}

impl Cli {
	pub async fn command_shell(&self, args: Args) -> Result<()> {
		// Create the run args.
		let args = super::run::Args {
			no_tui: false,
			package: args.package,
			package_args: args.package_args,
			retry: args.retry,
			run_args: args.run_args,
			target: "shell".to_owned(),
			trailing_args: args.trailing_args,
		};

		// Run!
		self.command_run(args).await?;

		Ok(())
	}
}
