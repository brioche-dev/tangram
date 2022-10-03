use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tangram::system::System;

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

pub async fn run(args: Args) -> Result<()> {
	// Create the builder.
	let builder = crate::builder().await?.lock_shared().await?;

	// Create the package.
	let package_hash = builder
		.checkin_package(&args.package, args.locked)
		.await
		.context("Failed to create the package.")?;

	// Add the args.
	let mut target_args = Vec::new();
	if let Some(system) = args.system {
		target_args.push(
			builder
				.add_expression(&tangram::expression::Expression::String(
					system.to_string().into(),
				))
				.await?,
		);
	};
	let target_args = builder
		.add_expression(&tangram::expression::Expression::Array(target_args))
		.await?;

	// Add the expression.
	let expression_hash = builder
		.add_expression(&tangram::expression::Expression::Target(
			tangram::expression::Target {
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
