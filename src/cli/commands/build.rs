use anyhow::Result;
use clap::Parser;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Parser)]
pub struct Args {
	#[clap(long, default_value = ".")]
	package: PathBuf,
	#[clap(long, default_value = "build")]
	export: String,
}

pub async fn run(args: Args) -> Result<()> {
	let client = crate::client::new().await?;
	let package = client.checkin_package(&args.package).await?;
	let expression = tangram::expression::Expression::Target(tangram::expression::Target {
		artifact_hash: package,
		lockfile: None,
		export: args.export,
		args: vec![tangram::expression::Expression::Map(BTreeMap::new())],
	});
	let value = client.evaluate(expression).await?;
	let value = serde_json::to_string_pretty(&value)?;
	println!("{value}");
	Ok(())
}
