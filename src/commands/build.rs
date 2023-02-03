use crate::{
	operation::{Operation, Target},
	package_specifier::PackageSpecifier,
	system::System,
	Cli,
};
use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser)]
#[command(about = "Build a package.")]
pub struct Args {
	#[arg(long)]
	locked: bool,

	#[arg(default_value = ".")]
	specifier: PackageSpecifier,

	#[arg(default_value = "default")]
	name: String,

	#[arg(long)]
	system: Option<System>,
}

impl Cli {
	pub async fn command_build(&self, args: Args) -> Result<()> {
		// Get the package hash.
		let package_hash = self
			.package_hash_for_specifier(&args.specifier, args.locked)
			.await
			.context("Failed to get the hash for the specifier.")?;

		// Create the target args.
		let target_args = self.create_target_args(args.system)?;

		// Create the operation.
		let operation = Operation::Target(Target {
			package: package_hash,
			name: args.name,
			args: target_args,
		});

		// Run the operation.
		let output = self
			.run(&operation)
			.await
			.context("Failed to run the operation.")?;

		// Print the output.
		println!("{output:?}");

		Ok(())
	}
}
