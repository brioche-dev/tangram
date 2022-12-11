use crate::{
	expression::{self, Expression},
	specifier::Specifier,
	system::System,
	Cli,
};
use anyhow::{Context, Result};
use clap::Parser;

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
		let target_args = cli.create_target_args(args.system).await?;

		// Add the expression.
		let expression_hash = cli
			.add_expression(&Expression::Target(expression::Target {
				package: package_hash,
				name: args.name,
				args: target_args,
			}))
			.await?;

		// Evaluate the expression.
		let output_hash = cli
			.evaluate(expression_hash, expression_hash)
			.await
			.context("Failed to evaluate the target expression.")?;

		// Print the output.
		println!("{output_hash}");

		Ok(())
	}
}
