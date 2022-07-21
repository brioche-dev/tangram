use anyhow::Result;
use clap::Parser;
use std::{collections::BTreeMap, path::PathBuf};
use tangram_core::{system::System, value::Value};

#[derive(Parser)]
pub struct Args {
	#[clap(long, default_value = ".")]
	package: PathBuf,
	#[clap(long, default_value = "build")]
	export: String,
}

pub async fn run(args: Args) -> Result<()> {
	todo!()
}
