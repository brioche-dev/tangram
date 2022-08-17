use anyhow::Result;
use clap::Parser;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Parser)]
pub struct Args {
	#[clap(long, default_value = ".")]
	package: PathBuf,
	#[clap(long, default_value = "build")]
	name: String,
}

pub async fn run(args: Args) -> Result<()> {
	// Create the client.
	let client = crate::client::new().await?;

	// Checkin the package.
	let package = client.checkin_package(&args.package).await?;

	// Evaluate the target.
	let expression = tangram::expression::Expression::Target(tangram::expression::Target {
		lockfile: tangram::lockfile::Lockfile(BTreeMap::new()),
		package,
		name: args.name,
		args: Box::new(tangram::expression::Expression::Array(vec![])),
	});
	let value = client.evaluate(expression).await?;

	// Print the value.
	let value = serde_json::to_string_pretty(&value)?;
	println!("{value}");

	Ok(())
}
