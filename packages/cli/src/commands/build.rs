use super::PackageArgs;
use crate::{
	tui::{self, Tui},
	Cli,
};
use std::path::PathBuf;
use tangram_client as tg;
use tangram_error::{Result, WrapErr};

/// Build a target.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// If this flag is set, then the command will exit immediately instead of waiting for the build's output.
	#[arg(short, long)]
	pub detach: bool,

	/// Disable the TUI.
	#[arg(long, default_value = "false")]
	pub no_tui: bool,

	/// The path to check out the output to.
	#[arg(short, long)]
	pub output: Option<PathBuf>,

	/// The package to build.
	#[arg(short, long, default_value = ".")]
	pub package: tangram_package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,

	/// The name of the target to build.
	#[arg(default_value = "default")]
	pub target: String,
}

impl Cli {
	pub async fn command_build(&self, args: Args) -> Result<()> {
		let client = self.client().await?;
		let client = client.as_ref();

		// Create the package.
		let (package, lock) = tangram_package::new(client, &args.package)
			.await
			.wrap_err("Failed to create the package.")?;

		// Create the target.
		let env = [(
			"TANGRAM_HOST".to_owned(),
			tg::System::host()?.to_string().into(),
		)]
		.into();
		let args_ = Vec::new();
		let host = tg::System::js();
		let path = tangram_package::ROOT_MODULE_FILE_NAME
			.to_owned()
			.try_into()
			.unwrap();
		let executable = tg::Symlink::with_package_and_path(&package, &path).into();
		let target = tg::target::Builder::new(host, executable)
			.lock(lock)
			.name(args.target.clone())
			.env(env)
			.args(args_)
			.build();

		// Build the target.
		let build = target.build(client).await?;
		eprintln!("{}", build.id(client).await?);

		// If the detach flag is set, then exit.
		if args.detach {
			println!("{}", build.id(client).await?);
			return Ok(());
		}

		// Create the TUI.
		let tui = !args.no_tui;
		let tui = if tui {
			Tui::start(client, &build, tui::Options::default())
				.await
				.ok()
		} else {
			None
		};

		// Wait for the build's result.
		let result = build.result(client).await;

		// Stop the TUI.
		if let Some(tui) = tui {
			tui.stop();
			tui.join().await?;
		}

		// Handle for an error that occurred while waiting for the build's result.
		let result = result.wrap_err("Failed to get the build result.")?;

		// Handle a failed build.
		let output = result.wrap_err("The build failed.")?;

		// Check out the output if requested.
		if let Some(path) = args.output {
			let artifact = tg::Artifact::try_from(output.clone())
				.wrap_err("Expected the output to be an artifact.")?;
			artifact
				.check_out(client, &path)
				.await
				.wrap_err("Failed to check out the artifact.")?;
		}

		// Print the output.
		println!("{output}");

		Ok(())
	}
}
