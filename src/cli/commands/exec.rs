use crate::{error::Result, Cli};

/// Build a target from the specified package and execute a command from its output.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
#[command(trailing_var_arg = true)]
pub struct Args {
	/// The name of the target to build.
	#[arg(short, long, default_value = "default")]
	pub target: String,

	#[command(flatten)]
	pub package_args: super::PackageArgs,

	#[command(flatten)]
	pub run_args: super::RunArgs,

	#[arg(default_value = ".")]
	pub package: tg::package::Specifier,

	pub trailing_args: Vec<String>,
}

impl Cli {
	pub async fn command_exec(&self, args: Args) -> Result<()> {
		// Create the run args.
		let args = super::run::Args {
			package: args.package,
			package_args: args.package_args,
			run_args: args.run_args,
			target: args.target,
			trailing_args: args.trailing_args,
		};

		// Run!
		self.command_run(args).await?;

		Ok(())
	}
}
