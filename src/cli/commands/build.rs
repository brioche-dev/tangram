use super::PackageArgs;
use crate::{
	error::{Result, WrapErr},
	Cli,
};
use std::path::PathBuf;
use tangram::{
	package::{self, Package, ROOT_MODULE_FILE_NAME},
	target::Target,
};

/// Build a target.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,

	#[arg(default_value = "default")]
	pub target: String,

	#[arg(short, long)]
	pub output: Option<PathBuf>,
}

impl Cli {
	pub async fn command_build(&self, args: Args) -> Result<()> {
		// Create the package.
		let package = Package::with_specifier(&self.tg, args.package)
			.await
			.wrap_err("Failed to get the package.")?;

		// Build the target.
		let env = Self::create_default_env()?;
		let args_ = Vec::new();
		let target = Target::new(
			&self.tg,
			package.artifact().block(),
			ROOT_MODULE_FILE_NAME.parse().unwrap(),
			args.target,
			env,
			args_,
		)
		.wrap_err("Failed to create the target.")?;
		let output = target
			.build(&self.tg)
			.await
			.wrap_err("Failed to build the target.")?;

		// Check out the output if requested.
		if let Some(path) = args.output {
			let artifact = output
				.as_artifact()
				.wrap_err("Expected the output to be an artifact.")?;
			artifact
				.check_out(&self.tg, &path)
				.await
				.wrap_err("Failed to check out the artifact.")?;
		}

		// Print the output.
		println!("{output}");

		Ok(())
	}
}
