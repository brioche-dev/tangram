use super::{PackageArgs, RunArgs};
use crate::{util::dirs::home_directory_path, Cli};
use std::{os::unix::process::CommandExt, path::PathBuf};
use tangram_client as tg;
use tangram_package::PackageExt;
use tg::{Result, Wrap, WrapErr};

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
		let package = tg::Package::with_specifier(self.client.as_ref(), args.package)
			.await
			.wrap_err("Failed to get the package.")?;

		// Create the target.
		let env = [(
			"TANGRAM_HOST".to_owned(),
			tg::System::host()?.to_string().into(),
		)]
		.into();
		let args_ = Vec::new();
		let host = tg::System::js();
		let executable = tg::package::ROOT_MODULE_FILE_NAME.to_owned().into();
		let target = tg::target::Builder::new(host, executable)
			.package(package)
			.name(args.target)
			.env(env)
			.args(args_)
			.build();

		// Build the target.
		let build = target.build(self.client.as_ref()).await?;

		// Wait for the build's output.
		let output = build
			.result(self.client.as_ref())
			.await?
			.wrap_err("The build failed.")?;

		// Get the output artifact.
		let artifact: tg::Artifact = output
			.try_into()
			.wrap_err("Expected the output to be an artifact.")?;

		// Get the path to the artifact.
		let artifact_path: PathBuf = home_directory_path()
			.wrap_err("Failed to find the user home directory.")?
			.join(".tangram")
			.join("artifacts")
			.join(artifact.id(self.client.as_ref()).await?.to_string());

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
			.wrap("Failed to execute the command."))
	}
}
