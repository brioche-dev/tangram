use super::PackageArgs;
use crate::Cli;
use tangram_client as tg;
use tg::Result;

/// Build the target named "test" from the specified package.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// If this flag is set, then the command will exit immediately instead of waiting for the build's output.
	#[arg(short, long)]
	pub detach: bool,

	/// The package to build.
	#[arg(short, long, default_value = ".")]
	pub package: tg::package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,
}

impl Cli {
	pub async fn command_test(&self, args: Args) -> Result<()> {
		// Create the build args.
		let args = super::build::Args {
			detach: args.detach,
			output: None,
			package: args.package,
			package_args: args.package_args,
			target: "test".to_owned(),
			non_interactive: true,
		};

		// Build!
		self.command_build(args).await?;

		Ok(())
	}
}
