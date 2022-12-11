use crate::{
	expression::{self, Expression},
	specifier::Specifier,
	system::System,
	util::path_exists,
	Cli,
};
use anyhow::{bail, Context, Result};
use clap::Parser;
use std::{os::unix::process::CommandExt, path::PathBuf};

#[derive(Parser, Debug)]
#[command(trailing_var_arg = true)]
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
	system: Option<System>,
}

impl Cli {
	pub(crate) async fn command_run(&self, args: Args) -> Result<()> {
		// Lock the cli.
		let cli = self.lock_shared().await?;

		// Get the package hash.
		let package_hash = cli
			.package_hash_for_specifier(&args.specifier, false)
			.await
			.context("Failed to get the hash for the specifier.")?;

		// Get the package manifest.
		let manifest = cli.get_package_manifest(package_hash).await?;

		// Get the package name.
		let package_name = manifest.name;

		// Get the executable path.
		let executable_path = args
			.executable_path
			.unwrap_or_else(|| PathBuf::from("bin").join(package_name));

		// Get the target name.
		let name = args.target.unwrap_or_else(|| "default".to_owned());

		// Create the target args.
		let target_args = cli.create_target_args(args.system).await?;

		// Add the expression.
		let input_hash = cli
			.add_expression(&Expression::Target(expression::Target {
				package: package_hash,
				name,
				args: target_args,
			}))
			.await?;

		// Evaluate the expression.
		let output_hash = cli
			.evaluate(input_hash, input_hash)
			.await
			.context("Failed to evaluate the target expression.")?;

		// Check out the artifact.
		let artifact_path = cli.checkout_to_artifacts(output_hash).await?;

		// Get the path to the executable.
		let executable_path = artifact_path.join(executable_path);

		// Verify the executable path exists.
		if !path_exists(&executable_path).await? {
			bail!(
				r#"No executable found at path "{}"."#,
				executable_path.display()
			);
		}

		// Drop the lock on the cli.
		drop(cli);

		// Exec the process.
		Err(std::process::Command::new(&executable_path)
			.args(args.trailing_args)
			.exec()
			.into())
	}
}
