use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tangram_core::{specifier::Specifier, system::System};

#[derive(Parser)]
#[command(long_about = "Build a package.")]
pub struct Args {
	#[arg(long)]
	locked: bool,

	#[arg(default_value = ".")]
	specifier: Specifier,

	#[arg(default_value = "default")]
	name: String,

	#[arg(long)]
	system: Option<System>,

	#[arg(long)]
	checkout: Option<PathBuf>,
}

impl Cli {
	pub(crate) async fn command_build(&self, args: Args) -> Result<()> {
		// Lock the builder.
		let builder = self.builder.lock_shared().await?;

		// Get the package hash.
		let package_hash = self
			.package_hash_for_specifier(&args.specifier, args.locked)
			.await
			.context("Failed to get the hash for the specifier.")?;

		// Create the target args.
		let target_args = self.create_target_args(args.system).await?;

		// Add the expression.
		let expression_hash = builder
			.add_expression(&tangram_core::expression::Expression::Target(
				tangram_core::expression::Target {
					package: package_hash,
					name: args.name,
					args: target_args,
				},
			))
			.await?;

		// Evaluate the expression.
		let output_hash = builder
			.evaluate(expression_hash, expression_hash)
			.await
			.context("Failed to evaluate the target expression.")?;

		// Print the output.
		println!("{output_hash}");

		// Checkout the built artifact if a path is provided.
		if let Some(checkout_path) = &args.checkout {
			builder
				.checkout(output_hash, checkout_path, None)
				.await
				.context("Failed to perform the checkout.")?;
		}

		Ok(())
	}
}
