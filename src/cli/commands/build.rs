use super::PackageArgs;
use crate::{
	error::{Result, WrapErr},
	Cli,
};
use std::path::PathBuf;
use tangram::{
	function::{self, Function},
	package::{self, Package, ROOT_MODULE_FILE_NAME},
};

/// Call a function.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,

	#[arg(default_value = "default")]
	pub function: String,

	#[arg(short, long)]
	pub output: Option<PathBuf>,
}

impl Cli {
	pub async fn command_build(&self, args: Args) -> Result<()> {
		// Create the package.
		let package = Package::with_specifier(&self.tg, args.package)
			.await
			.wrap_err("Failed to get the package.")?;

		// Call the function.
		let env = Self::create_default_env()?;
		let args_ = Vec::new();
		let function = Function::new(
			&self.tg,
			package.hash(),
			ROOT_MODULE_FILE_NAME.parse().unwrap(),
			function::Kind::Function,
			args.function,
			env,
			args_,
		)
		.wrap_err("Failed to create the function.")?;
		let output = function
			.call(&self.tg)
			.await
			.wrap_err("The function call failed.")?;

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
