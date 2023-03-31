use super::{PackageArgs, RunArgs};
use crate::{
	error::{Result, WrapErr},
	Cli,
};
use std::os::unix::process::CommandExt;
use tangram::{
	artifact::Artifact,
	function::Function,
	operation::{Call, Operation},
	package,
	path::Path,
};

/// Build a package and run an executable from its output.
#[derive(Debug, clap::Args)]
#[command(trailing_var_arg = true)]
pub struct Args {
	/// The package to build.
	#[arg(short, long, default_value = ".")]
	pub package: package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,

	#[command(flatten)]
	pub run_args: RunArgs,

	/// The export to build.
	#[arg(default_value = "default")]
	pub export: String,

	/// Arguments to pass to the executable.
	pub trailing_args: Vec<String>,
}

impl Cli {
	pub async fn command_run(&self, args: Args) -> Result<()> {
		// Resolve the package specifier.
		let package_identifier = self.tg.resolve_package(&args.package, None).await?;

		// Create the package instance.
		let package_instance_hash = self
			.tg
			.clone()
			.create_package_instance(&package_identifier, args.package_args.locked)
			.await?;

		// Run the operation.
		let function = Function {
			package_instance_hash,
			name: args.export,
		};
		let context = Self::create_default_context()?;
		let args_ = Vec::new();
		let operation = Operation::Call(Call {
			function,
			context,
			args: args_,
		});
		let output = operation.run(&self.tg).await?;

		// Get the output artifact.
		let artifact_hash = output
			.into_artifact()
			.wrap_err("Expected the output to be an artifact.")?;

		// Get the artifact.
		let artifact = self.tg.get_artifact_local(artifact_hash)?;

		// Check out the artifact.
		let artifact_path = self.tg.check_out_internal(artifact_hash).await?;

		// Get the executable path.
		let executable_path = if let Some(executable_path) = args.run_args.executable_path {
			executable_path
		} else {
			match artifact {
				// If the artifact is a file or symlink, then the executable path should be empty.
				Artifact::File(_) | Artifact::Symlink(_) => Path::new(),

				// If the artifact is a directory, then the executable path should be "run".
				Artifact::Directory(_) => "run".parse().unwrap(),
			}
		};

		// Get the path to the executable.
		let executable_path = artifact_path.join(executable_path.to_string());

		// Exec the process.
		Err(std::process::Command::new(executable_path)
			.args(args.trailing_args)
			.exec()
			.into())
	}
}
