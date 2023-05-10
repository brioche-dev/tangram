use super::PackageArgs;
use crate::{error::Result, Cli};
use tangram::package;

/// Build a package's "test" export and run it.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
#[command(trailing_var_arg = true)]
pub struct Args {
	/// The package to build.
	#[arg(short, long, default_value = ".")]
	pub package: package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,

	#[arg(default_value = "test")]
	pub function: String,
}

impl Cli {
	pub async fn command_test(&self, args: Args) -> Result<()> {
		// Create the build args.
		let args = super::build::Args {
			package: args.package,
			package_args: args.package_args,
			function: args.function,
			output: None,
		};

		// Build!
		self.command_build(args).await?;

		Ok(())
	}
}
