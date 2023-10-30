use super::PackageArgs;
use crate::{
	ui::{self, DevTty},
	Cli,
};
use std::path::PathBuf;
use tangram_client as tg;
use tangram_package::PackageExt;
use tg::{Result, WrapErr};

/// Build a target.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// If this flag is set, then the command will exit immediately instead of waiting for the build's output.
	#[arg(short, long)]
	pub detach: bool,

	/// The path to check out the output to.
	#[arg(short, long)]
	pub output: Option<PathBuf>,

	/// The package to build.
	#[arg(short, long, default_value = ".")]
	pub package: tg::package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,

	/// The name of the target to build.
	#[arg(default_value = "default")]
	pub target: String,

	/// Enable an interactive TUI.
	#[arg(long, default_value = "false")]
	pub non_interactive: bool,
}

impl Cli {
	pub async fn command_build(&self, args: Args) -> Result<()> {
		let client = self.client().await?;

		// Create the package.
		let package = tg::Package::with_specifier(client.as_ref(), args.package)
			.await
			.wrap_err("Failed to get the package.")?;

		// Create the target.
		let env = [(
			"TANGRAM_HOST".to_owned(),
			tg::System::host()?.to_string().into(),
		)]
		.into();
		let args_ = Vec::new();
		let host = tg::System::js();
		let executable = tg::package::ROOT_MODULE_FILE_NAME.to_owned().into();
		let target = tg::target::Builder::new(host, executable)
			.package(package)
			.name(args.target.clone())
			.env(env)
			.args(args_)
			.build();

		// Build the target.
		let build = target.build(client.as_ref()).await?;

		// If the detach flag is set, then exit.
		if args.detach {
			println!("{}", build.id());
			return Ok(());
		}

		// Create the ui.
		let mut ui = None;
		if !args.non_interactive {
			if let Ok(tty) = DevTty::open() {
				ui = Some(ui::ui(client.as_ref(), tty, build.clone(), args.target.clone()));
			}
		}

		// Wait for the build's output.
		let output = build
			.result(client.as_ref())
			.await
			.wrap_err("Failed to get the build result.")?
			.wrap_err("The build failed.")?;

		// Check out the output if requested.
		if let Some(path) = args.output {
			let artifact = tg::Artifact::try_from(output.clone())
				.wrap_err("Expected the output to be an artifact.")?;
			artifact
				.check_out(client.as_ref(), &path)
				.await
				.wrap_err("Failed to check out the artifact.")?;
		}

		// Print the output.
		println!("{output}");

		drop(ui);
		Ok(())
	}
}
