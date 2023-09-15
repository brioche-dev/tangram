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
	/// The package to build.
	#[arg(short, long, default_value = ".")]
	pub package: tg::package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,

	/// The name of the target to build.
	#[arg(default_value = "default")]
	pub target: String,

	/// The path to check out the output to.
	#[arg(short, long)]
	pub output: Option<PathBuf>,
}

impl Cli {
	pub async fn command_build(&self, args: Args) -> Result<()> {
		// Create the package.
		let package = tg::Package::with_specifier(&self.client, args.package)
			.await
			.wrap_err("Failed to get the package.")?;

		// Create the target.
		let env = [(
			"host".to_owned(),
			tg::Handle::with_value(tg::System::host()?.to_string().into()),
		)]
		.into();
		let args_ = Vec::new();
		let target = tg::Target::new(
			package,
			tg::package::ROOT_MODULE_FILE_NAME.parse().unwrap(),
			args.target,
			env,
			args_,
		);

		// Get the output.
		let output = target
			.evaluate(&self.client)
			.await
			.wrap_err("Failed to build the target.")?;

		if let Some(path) = args.output {
			// Check out the output if requested.
			let artifact = tg::Artifact::try_from(output)
				.wrap_err("Expected the output to be an artifact.")?;
			artifact
				.check_out(&self.client, &path)
				.await
				.wrap_err("Failed to check out the artifact.")?;
		} else {
			// TODO: Print the output.
			// println!("{output}");
		}

		Ok(())
	}
}
