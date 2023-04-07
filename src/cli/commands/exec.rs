use crate::{error::Result, Cli};
use tangram::package;

/// Build a package and run an executable from its output.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
#[command(trailing_var_arg = true)]
pub struct Args {
	/// The name of the function to call.
	#[arg(short, long, default_value = "default")]
	pub function: String,

	#[command(flatten)]
	pub package_args: super::PackageArgs,

	#[command(flatten)]
	pub run_args: super::RunArgs,

	#[arg(default_value = ".")]
	pub package: package::Specifier,

	pub trailing_args: Vec<String>,
}

impl Cli {
	pub async fn command_exec(&self, args: Args) -> Result<()> {
		// Create the run args.
		let args = super::run::Args {
			package: args.package,
			package_args: args.package_args,
			run_args: args.run_args,
			function: args.function,
			trailing_args: args.trailing_args,
		};

		// Run!
		self.command_run(args).await?;

		Ok(())
	}
}
