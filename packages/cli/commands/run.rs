use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use std::{os::unix::process::CommandExt, path::PathBuf};
use tangram_core::specifier::{self, Specifier};
use tangram_core::system::System;

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
		// Lock the builder.
		let builder = self.builder.lock_shared().await?;

		// Get the package hash.
		let package_hash = match args.specifier {
			Specifier::Path(specifier::Path { path }) => {
				// Create the package.
				builder
					.checkin_package(&self.api_client, &path, args.locked)
					.await
					.context("Failed to create the package.")?
			},

			Specifier::Registry(specifier::Registry {
				package_name,
				version,
			}) => {
				// Get the package from the registry.
				let version = version.context("A version is required.")?;
				self.api_client
					.get_package_version(&package_name, &version)
					.await
					.with_context(|| {
						format!(r#"Failed to get the package "{package_name}" from the registry."#)
					})?
			},
		};

		// Get the package manifest.
		let manifest = builder.get_package_manifest(package_hash).await?;

		// Get the package name.
		let package_name = manifest.name;

		// Get the executable path.
		let executable_path = args
			.executable_path
			.unwrap_or_else(|| PathBuf::from("bin").join(package_name));

		// Get the target name.
		let name = args.target.unwrap_or_else(|| "default".to_owned());

		// Create the target args.
		let target_args = self.create_target_args(args.system).await?;

		// Add the expression.
		let input_hash = builder
			.add_expression(&tangram_core::expression::Expression::Target(
				tangram_core::expression::Target {
					package: package_hash,
					name,
					args: target_args,
				},
			))
			.await?;

		// Evaluate the expression.
		let output_hash = builder
			.evaluate(input_hash, input_hash)
			.await
			.context("Failed to evaluate the target expression.")?;

		// Check out the artifact.
		let artifact_path = builder.checkout_to_artifacts(output_hash).await?;

		// Get the path to the executable.
		let executable_path = artifact_path.join(executable_path);

		// Drop the lock on the builder.
		drop(builder);

		// Exec the process.
		Err(std::process::Command::new(&executable_path)
			.args(args.trailing_args)
			.exec()
			.into())
	}
}
