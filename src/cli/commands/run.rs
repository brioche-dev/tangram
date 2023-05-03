use super::{PackageArgs, RunArgs};
use crate::{
	error::{Result, WrapErr},
	Cli,
};
use std::os::unix::process::CommandExt;
use tangram::{
	artifact::Artifact,
	function::Function,
	operation::Call,
	package::{self, Package, ROOT_MODULE_FILE_NAME},
	util::fs,
};

/// Build a package and run an executable from its output.
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

	/// The name of the function to call.
	#[arg(default_value = "default")]
	pub function: String,

	/// Arguments to pass to the executable.
	pub trailing_args: Vec<String>,
}

impl Cli {
	pub async fn command_run(&self, args: Args) -> Result<()> {
		// Get the package.
		let package = Package::with_specifier(&self.tg, args.package).await?;

		// Create the package instance.
		let package_instance = package
			.instantiate(&self.tg)
			.await
			.wrap_err("Failed to create the package instance.")?;

		// Run the operation.
		let function = Function::new(
			&package_instance,
			ROOT_MODULE_FILE_NAME.into(),
			args.function,
		);
		let env = Self::create_default_env()?;
		let args_ = Vec::new();
		let call = Call::new(&self.tg, function, env, args_)
			.await
			.wrap_err("Failed to create the call.")?;
		let output = call.run(&self.tg).await?;

		// Get the output artifact.
		let artifact = output
			.into_artifact()
			.wrap_err("Expected the output to be an artifact.")?;

		// Check out the artifact.
		let artifact_path = artifact.check_out_internal(&self.tg).await?;

		// Get the executable path.
		let executable_path = if let Some(executable_path) = args.run_args.executable_path {
			// Resolve the argument as a path relative to the artifact.
			artifact_path.join(fs::PathBuf::from(executable_path))
		} else {
			match artifact {
				// If the artifact is a file or symlink, then the executable path should be the artifact itself.
				Artifact::File(_) | Artifact::Symlink(_) => artifact_path,

				// If the artifact is a directory, then the executable path should be "run".
				Artifact::Directory(_) => artifact_path.join("run"),
			}
		};

		// Exec the process.
		let error = std::process::Command::new(executable_path)
			.args(args.trailing_args)
			.exec();
		Err(error.into())
	}
}
