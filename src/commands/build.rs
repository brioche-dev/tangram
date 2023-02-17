use crate::{
	function::Function,
	operation::{Call, Operation},
	package, Cli,
};
use anyhow::{Context, Result};
use std::sync::Arc;

/// Call a function.
#[derive(clap::Args)]
pub struct Args {
	#[arg(long)]
	locked: bool,

	#[arg(default_value = ".")]
	package_specifier: package::Specifier,

	#[arg(default_value = "default")]
	name: String,
}

impl Cli {
	pub async fn command_build(self: &Arc<Self>, args: Args) -> Result<()> {
		// Resolve the package specifier.
		let package_identifier = self.resolve_package(&args.package_specifier, None).await?;

		// Create the package instance.
		let package_instance_hash = self
			.create_package_instance(&package_identifier, args.locked)
			.await
			.context("Failed to create the package instance.")?;

		// Create the operation.
		let function = Function {
			package_instance_hash,
			name: args.name,
		};
		let context = self.create_default_context()?;
		let operation = Operation::Call(Call {
			function,
			context,
			args: vec![],
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
