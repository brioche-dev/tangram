use crate::{
	operation::{Operation, Target},
	specifier::Specifier,
	system::System,
	util::path_exists,
	Cli,
};
use anyhow::{bail, Context, Result};
use clap::Parser;
use std::{os::unix::process::CommandExt, path::PathBuf};

#[derive(Parser, Debug)]
#[command(
	about = "Build a package and run an executable from its output.",
	trailing_var_arg = true
)]
pub struct Args {
	#[arg(long)]
	pub executable_path: Option<PathBuf>,
	#[arg(long)]
	pub locked: bool,
	#[arg(long)]
	pub target: Option<String>,
	#[arg(default_value = ".")]
	pub specifier: Specifier,
	pub trailing_args: Vec<String>,
	#[arg(long)]
	pub system: Option<System>,
}

impl Cli {
	pub async fn command_run(&self, args: Args) -> Result<()> {
		// Get the package hash.
		let package_hash = self
			.package_hash_for_specifier(&args.specifier, false)
			.await
			.context("Failed to get the hash for the specifier.")?;

		// Get the package manifest.
		let manifest = self.get_package_manifest(package_hash).await?;

		// Get the package name.
		let package_name = manifest.name;

		// Get the executable path.
		let executable_path = args
			.executable_path
			.unwrap_or_else(|| PathBuf::from("bin").join(package_name));

		// Get the target name.
		let name = args.target.unwrap_or_else(|| "default".to_owned());

		// Create the target args.
		let target_args = self.create_target_args(args.system)?;

		// Create the operation.
		let operation = Operation::Target(Target {
			package: package_hash,
			name,
			args: target_args,
		});

		// Run the operation.
		let output = self
			.run(&operation)
			.await
			.context("Failed to run the operation.")?;

		// Get the output artifact.
		let output_artifact_hash = output
			.into_artifact()
			.context("Expected the output to be an artifact.")?;

		// Check out the artifact.
		let artifact_path = self.checkout_internal(output_artifact_hash).await?;

		// Get the path to the executable.
		let executable_path = artifact_path.join(executable_path);

		// Verify the executable path exists.
		if !path_exists(&executable_path).await? {
			bail!(
				r#"No executable found at path "{}"."#,
				executable_path.display()
			);
		}

		// Exec the process.
		Err(std::process::Command::new(&executable_path)
			.args(args.trailing_args)
			.exec()
			.into())
	}
}
