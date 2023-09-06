use super::PackageArgs;
use crate::{
	error::{Result, WrapErr},
	Cli,
};
use std::path::PathBuf;

/// Build a target.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: tg::package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,

	#[arg(default_value = "default")]
	pub target: String,

	#[arg(short, long)]
	pub output: Option<PathBuf>,
}

impl Cli {
	pub async fn command_build(&self, args: Args) -> Result<()> {
		// Create the package.
		let package = tg::Package::with_specifier(&self.tg, args.package)
			.await
			.wrap_err("Failed to get the package.")?;

		// Build the target.
		let env = [(
			"host".to_owned(),
			tg::Value::from(tg::System::host()?.to_string()),
		)]
		.into();
		let args_ = Vec::new();
		let target = tg::Target::new(
			package.block().clone(),
			tg::package::ROOT_MODULE_FILE_NAME.parse().unwrap(),
			args.target,
			env,
			args_,
		);
		let output = target
			.build(&self.tg)
			.await
			.wrap_err("Failed to build the target.")?;

		// Check out the output if requested.
		if let Some(path) = args.output {
			let artifact = output
				.as_artifact()
				.wrap_err("Expected the output to be an artifact.")?;
			artifact
				.check_out(&self.tg, &path)
				.await
				.wrap_err("Failed to check out the artifact.")?;
		}

		// Print the output.
		println!("{output}");

		Ok(())
	}
}
