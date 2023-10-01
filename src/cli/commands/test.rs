use super::PackageArgs;
use crate::{Cli, Result};

/// Build the target named "test" from the specified package.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The package to build.
	#[arg(short, long, default_value = ".")]
	pub package: tg::package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,

	#[arg(default_value = "test")]
	pub target: String,
}

impl Cli {
	pub async fn command_test(&self, args: Args) -> Result<()> {
		// Create the build args.
		let args = super::build::Args {
			package: args.package,
			package_args: args.package_args,
			target: args.target,
			output: None,
		};

		// Build!
		self.command_build(args).await?;

		Ok(())
	}
}
