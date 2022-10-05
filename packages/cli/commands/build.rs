use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tangram_core::system::System;

#[derive(Parser)]
pub struct Args {
	#[arg(long)]
	locked: bool,
	#[arg(default_value = ".")]
	package: PathBuf,
	#[arg(default_value = "default")]
	name: String,
	#[arg(long)]
	system: Option<System>,
}

impl Cli {
	pub(crate) async fn command_build(&self, args: Args) -> Result<()> {
		// Lock the builder.
		let builder = self.builder.lock_shared().await?;

		// Create the package.
		let package_hash = builder
			.checkin_package(&args.package, args.locked)
			.await
			.context("Failed to create the package.")?;

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
		println!("{expression_hash} => {output_hash}");

		Ok(())
	}
}
