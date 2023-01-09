use crate::{
	operation::{Operation, Target},
	specifier::Specifier,
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
	specifier: Specifier,

	#[arg(default_value = "default")]
	name: String,

	#[arg(long)]
	system: Option<System>,
}

impl Cli {
	pub(crate) async fn command_build(&self, args: Args) -> Result<()> {
		// Lock the cli.
		let cli = self.lock_shared().await?;

		// Get the package hash.
		let package_hash = cli
			.package_hash_for_specifier(&args.specifier, args.locked)
			.await
			.context("Failed to get the hash for the specifier.")?;

		// Create the target args.
		let target_args = cli.create_target_args(args.system)?;

		// Create the operation.
		let operation = Operation::Target(Target {
			package: package_hash,
			name: args.name,
			args: target_args,
		});

		// Run the operation.
		let output = cli
			.run(&operation)
			.await
			.context("Failed to run the operation.")?;

		// Print the output.
		println!("{output:?}");

		Ok(())
	}
}
