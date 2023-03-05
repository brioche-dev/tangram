use crate::{
	function::Function,
	operation::{Call, Operation},
	os, package,
	path::Path,
	Cli,
};
use anyhow::{bail, Context, Result};
use std::{os::unix::process::CommandExt, sync::Arc};

#[derive(clap::Args)]
#[command(
	about = "Build a package and run an executable from its output.",
	trailing_var_arg = true
)]
pub struct Args {
	#[arg(long)]
	pub executable_path: Option<Path>,
	#[arg(long)]
	pub locked: bool,
	#[arg(long)]
	pub export: Option<String>,

	#[arg(default_value = ".")]
	pub package_specifier: package::Specifier,

	pub trailing_args: Vec<String>,
}

impl Cli {
	pub async fn command_run(self: &Arc<Self>, args: Args) -> Result<()> {
		// Resolve the package specifier.
		let package_identifier = self.resolve_package(&args.package_specifier, None).await?;

		// Get the package instance hash.
		let package_instance_hash = self
			.create_package_instance(&package_identifier, args.locked)
			.await?;

		// Get the export name.
		let name = args.export.unwrap_or_else(|| "default".to_owned());

		// Create the operation.
		let function = Function {
			package_instance_hash,
			name,
		};
		let context = self.create_default_context()?;
		let operation = Operation::Call(Call {
			function,
			context,
			args: vec![],
		});

		// Run the operation.
		let output = self
			.run(&operation)
			.await
			.context("Failed to run the operation.")?;

		// Get the output artifact.
		let output_artifact_hash = output
			.into_artifact()
			.context("Expected the output to be an artifact.")?;

		// Check out the artifact.
		let artifact_path = self.check_out_internal(output_artifact_hash).await?;

		// Get the executable path.
		let executable_path = args
			.executable_path
			.unwrap_or_else(|| "run".parse().unwrap());

		// Get the path to the executable.
		let executable_path = artifact_path.join(executable_path.to_string());

		// Verify the executable path exists.
		if !os::fs::exists(&executable_path).await? {
			bail!(
				r#"No executable found at path "{}"."#,
				executable_path.display()
			);
		}

		// Exec the process.
		Err(std::process::Command::new(&executable_path)
			.args(args.trailing_args)
			.exec()
			.into())
	}
}
