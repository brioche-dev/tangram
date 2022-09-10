use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(trailing_var_arg = true)]
pub struct Args {
	#[clap(long, takes_value = false)]
	locked: bool,
	#[clap(default_value = ".")]
	package: String,
	#[clap(long)]
	pub executable_path: Option<PathBuf>,
}

pub async fn run(args: Args) -> Result<()> {
	let executable_subpath = if let Some(executable_subpath) = args.executable_path {
		executable_subpath
	} else {
		PathBuf::from("bin/shell")
	};

	super::run::run(super::run::Args {
		locked: args.locked,
		package: args.package,
		name: "shell".to_owned(),
		executable_path: Some(executable_subpath),
		trailing_args: vec![],
	})
	.await?;

	Ok(())
}
