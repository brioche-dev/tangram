use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tangram::{
	artifact::Artifact,
	expression,
	hash::Hash,
	object::ObjectHash,
	server::{fragment::Fragment, Server},
};
use tracing::Instrument;

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
	let client = crate::client::new()
		.await
		.context("Failed to create the client.")?;

	// Checkin the package.
	let package = client
		.checkin_package(&args.package, args.locked)
		.await
		.context("Failed to check in the package.")?;

	// Evaluate the target.
	let expression = tangram::expression::Expression::Target(tangram::expression::Target {
		lockfile: None,
		package,
		name: args.name,
		args: vec![],
	});
	let value = client
		.evaluate(expression)
		.await
		.context("Failed to evaluate the target expression.")?;

	// Print the value.
	let value = serde_json::to_string_pretty(&value).context("Failed to serialize the value.")?;
	println!("{value}");

	Ok(())
}
