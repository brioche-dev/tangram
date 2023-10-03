use super::{PackageArgs, RunArgs};
use crate::{return_error, Cli, Result, WrapErr};
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
		// Create the package.
		let package = tg::Package::with_specifier(&self.client, args.package)
			.await
			.wrap_err("Failed to get the package.")?;

		// Create the task.
		let env = [(
			"TANGRAM_HOST".to_owned(),
			tg::Value::String(tg::System::host()?.to_string()),
		)]
		.into();
		let args_ = Vec::new();
		let host = tg::System::js();
		let executable = tg::package::ROOT_MODULE_FILE_NAME.to_owned().into();
		let task = tg::task::Builder::new(host, executable)
			.package(package)
			.target(args.target)
			.env(env)
			.args(args_)
			.build();

		// Run the task.
		let run = task.run(&self.client).await?;
		let Some(output) = run.output(&self.client).await? else {
			return_error!("The build failed.");
		};

		// Get the output artifact.
		let artifact: tg::Artifact = output
			.try_into()
			.wrap_err("Expected the output to be an artifact.")?;

		// Get the path to the artifact.
		let artifact_path: PathBuf = tg::util::dirs::home_directory_path()
			.wrap_err("Failed to find the user home directory.")?
			.join(".tangram")
			.join("artifacts")
			.join(artifact.id(&self.client).await?.to_string());

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
