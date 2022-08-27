use anyhow::Result;
use clap::Parser;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Parser)]
pub struct Args {
	#[clap(long, default_value = ".")]
	package: PathBuf,
	#[clap(long, default_value = "build")]
	name: String,
	#[clap(long, takes_value = false)]
	locked: bool,
}

pub async fn run(args: Args) -> Result<()> {
	// Create the client.
	let client = crate::client::new().await?;

	// Checkin the package.
	let package = client.checkin_package(&args.package, args.locked).await?;

	// Evaluate the target.
	let expression = tangram::expression::Expression::Target(tangram::expression::Target {
		lockfile: tangram::lockfile::Lockfile(BTreeMap::new()),
		package,
		name: args.name,
		args: vec![],
	});
	let value = client.evaluate(expression).await?;

	// Print the value.
	let value = serde_json::to_string_pretty(&value)?;
	println!("{value}");

	Ok(())
}
