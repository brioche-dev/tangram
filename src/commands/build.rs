use crate::Cli;
use anyhow::{Context, Result};
use tangram::{
	function::Function,
	operation::{Call, Operation},
	os, package,
};

/// Call a function.
#[derive(clap::Args)]
pub struct Args {
	#[arg(long)]
	locked: bool,

	#[arg(default_value = ".")]
	package_specifier: package::Specifier,

	#[arg(default_value = "default")]
	name: String,

	#[arg(long)]
	checkout: bool,
}

impl Cli {
	pub async fn command_build(&self, args: Args) -> Result<()> {
		// Resolve the package specifier.
		let package_identifier = self
			.tg
			.resolve_package(&args.package_specifier, None)
			.await?;

		// Create the package instance.
		let package_instance_hash = self
			.tg
			.create_package_instance(&package_identifier, args.locked)
			.await
			.context("Failed to create the package instance.")?;

		// Create the operation.
		let function = Function {
			package_instance_hash,
			name: args.name,
		};
		let context = Self::create_default_context()?;
		let operation = Operation::Call(Call {
			function,
			context,
			args: vec![],
		});

		// Run the operation.
		let output = self
			.tg
			.run(&operation)
			.await
			.context("Failed to run the operation.")?;

		// Print the output.
		println!("{output:?}");

		// Check out the output if requested.
		if args.checkout {
			let artifact_hash = output
				.as_artifact()
				.context("Expected the output to be an artifact.")?;
			self.tg
				.check_out_external(*artifact_hash, os::Path::new("output"))
				.await?;
		}

		Ok(())
	}
}
