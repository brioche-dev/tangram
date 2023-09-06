use super::{PackageArgs, RunArgs};
use crate::{
	error::{Result, WrapErr},
	Cli,
};
use std::{os::unix::process::CommandExt, path::PathBuf};

/// Build the specified target from a package and execute a command from its output.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
#[command(trailing_var_arg = true)]
pub struct Args {
	/// The package to build.
	#[arg(short, long, default_value = ".")]
	pub package: tg::package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,

	#[command(flatten)]
	pub run_args: RunArgs,

	/// The name of the target to build.
	#[arg(default_value = "default")]
	pub target: String,

	/// Arguments to pass to the executable.
	pub trailing_args: Vec<String>,
}

impl Cli {
	pub async fn command_run(&self, args: Args) -> Result<()> {
		// Get the package.
		let package = tg::Package::with_specifier(&self.tg, args.package)
			.await
			.wrap_err("Failed to get the package.")?;

		// Build.
		let env = [(
			"host".to_owned(),
			tg::Value::from(tg::System::host()?.to_string()),
		)]
		.into();
		let args_ = Vec::new();
		let target = tg::Target::new(
			package,
			tg::package::ROOT_MODULE_FILE_NAME.parse().unwrap(),
			args.target,
			env,
			args_,
		);
		let output = target.build(&self.tg).await?;

		// Get the output artifact.
		let artifact = output
			.into_artifact()
			.wrap_err("Expected the output to be an artifact.")?;

		// Check out the artifact.
		let artifact_path = artifact.check_out_internal(&self.tg).await?;

		// Get the executable path.
		let executable_path = if let Some(executable_path) = args.run_args.executable_path {
			// Resolve the argument as a path relative to the artifact.
			artifact_path.join(PathBuf::from(executable_path))
		} else {
			match artifact {
				// If the artifact is a file or symlink, then the executable path should be the artifact itself.
				tg::artifact::Artifact::File(_) | tg::artifact::Artifact::Symlink(_) => {
					artifact_path
				},

				// If the artifact is a directory, then the executable path should be `.tangram/run`.
				tg::artifact::Artifact::Directory(_) => artifact_path.join(".tangram/run"),
			}
		};

		// Exec.
		Err(std::process::Command::new(executable_path)
			.args(args.trailing_args)
			.exec()
			.into())
	}
}
