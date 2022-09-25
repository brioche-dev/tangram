use crate::config::Config;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tangram::{client::Client, system::System};

#[derive(Parser)]
pub struct Args {
	#[clap(long, takes_value = false)]
	locked: bool,
	#[clap(default_value = ".")]
	package: PathBuf,
	#[clap(default_value = "default")]
	name: String,
	#[clap(long)]
	system: Option<System>,
}

pub async fn run(args: Args) -> Result<()> {
	// Read the config.
	let config = Config::read().await.context("Failed to read the config.")?;

	// Create the client.
	let client = Client::new_with_config(config.client)
		.await
		.context("Failed to create the client.")?;

	// Checkin the package.
	let package = client
		.checkin_package(&args.package, args.locked)
		.await
		.context("Failed to check in the package.")?;

	// Add the args.
	let mut target_args = Vec::new();
	if let Some(system) = args.system {
		target_args.push(
			client
				.add_expression(&tangram::expression::Expression::String(
					system.to_string().into(),
				))
				.await?,
		);
	};
	let target_args = client
		.add_expression(&tangram::expression::Expression::Array(target_args))
		.await?;

	// Add the expression.
	let expression_hash = client
		.add_expression(&tangram::expression::Expression::Target(
			tangram::expression::Target {
				lockfile: None,
				package,
				name: args.name,
				args: target_args,
			},
		))
		.await?;

	// Evaluate the expression.
	let output_hash = client
		.evaluate(expression_hash)
		.await
		.context("Failed to evaluate the target expression.")?;

	// Print the output.
	let output = client.get_expression(output_hash).await?;
	let output_json =
		serde_json::to_string_pretty(&output).context("Failed to serialize the expression.")?;
	println!("{expression_hash} => {output_json}");

	Ok(())
}
