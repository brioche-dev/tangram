use super::PackageArgs;
use crate::{return_error, Cli, Result, WrapErr};
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

		// Create the task.
		let env = [(
			"TANGRAM_HOST".to_owned(),
			tg::System::host()?.to_string().into(),
		)]
		.into();
		let args_ = Vec::new();
		let host = tg::System::js();
		let executable = tg::package::ROOT_MODULE_FILE_NAME.to_owned().into();
		let task = tg::task::Builder::new(host, executable)
			.package(package)
			.target(args.target)
			.env(env)
			.args(args_)
			.build();

		// Run the task.
		let run = task.run(&self.client).await?;
		let Some(output) = run.output(&self.client).await? else {
			return_error!("The build failed.");
		};

		if let Some(path) = args.output {
			// Check out the output if requested.
			let artifact = tg::Artifact::try_from(output)
				.wrap_err("Expected the output to be an artifact.")?;
			artifact
				.check_out(&self.client, &path)
				.await
				.wrap_err("Failed to check out the artifact.")?;
		} else {
			// Print the output.
			println!("{output:?}");
		}

		Ok(())
	}
}
