use super::{PackageArgs, RunArgs};
use crate::{
	tui::{self, Tui},
	util::dirs::home_directory_path,
	Cli,
};
use std::{os::unix::process::CommandExt, path::PathBuf};
use tangram_client as tg;
use tangram_error::{Result, Wrap, WrapErr};

/// Build the specified target from a package and execute a command from its output.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
#[command(trailing_var_arg = true)]
pub struct Args {
	/// Disable the TUI.
	#[arg(long, default_value = "false")]
	pub no_tui: bool,

	/// The package to build.
	#[arg(short, long, default_value = ".")]
	pub package: tangram_package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,

	#[command(flatten)]
	pub run_args: RunArgs,

	/// The name of the target to build.
	#[arg(default_value = "default")]
	pub target: String,

	/// Arguments to pass to the executable.
	pub trailing_args: Vec<String>,
}

impl Cli {
	pub async fn command_run(&self, args: Args) -> Result<()> {
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

		// Create the TUI.
		let tui = !args.no_tui;
		let tui = if tui {
			Tui::start(client, &build, tui::Options::default())
				.await
				.ok()
		} else {
			None
		};

		// Wait for the build's output.
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

		// Get the output artifact.
		let artifact: tg::Artifact = output
			.try_into()
			.wrap_err("Expected the output to be an artifact.")?;

		// Get the path to the artifact.
		let artifact_path: PathBuf = home_directory_path()
			.wrap_err("Failed to find the user home directory.")?
			.join(".tangram")
			.join("artifacts")
			.join(artifact.id(client).await?.to_string());

		// Get the executable path.
		let executable_path = if let Some(executable_path) = args.run_args.executable_path {
			// Resolve the argument as a path relative to the artifact.
			artifact_path.join(PathBuf::from(executable_path))
		} else {
			match artifact {
				// If the artifact is a file or symlink, then the executable path should be the artifact itself.
				tg::artifact::Artifact::File(_) | tg::artifact::Artifact::Symlink(_) => {
					artifact_path
				},

				// If the artifact is a directory, then the executable path should be `.tangram/run`.
				tg::artifact::Artifact::Directory(_) => artifact_path.join(".tangram/run"),
			}
		};

		// Exec.
		Err(std::process::Command::new(executable_path)
			.args(args.trailing_args)
			.exec()
			.wrap("Failed to execute the command."))
	}
}
