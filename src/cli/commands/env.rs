use crate::{
	error::{Result, WrapErr},
	Cli,
};
use itertools::Itertools;
use tangram::{
	function::Function,
	operation::{Call, Operation},
	package,
};

#[derive(clap::Args)]
pub struct Args {}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_env(&self, _args: Args) -> Result<()> {
		// Read the config.
		let config = self.read_config().await?.unwrap_or_default();

		// Get the current working directory.
		let working_directory_path =
			std::env::current_dir().wrap_err("Failed to get the working directory.")?;

		// Get the autoenv path for the working directory path.
		let Some(autoenv_paths) = config.autoenvs.as_ref() else {
			return Ok(());
		};
		let mut autoenv_paths = autoenv_paths
			.iter()
			.filter(|path| working_directory_path.starts_with(path))
			.collect_vec();
		autoenv_paths.sort_by_key(|path| path.components().count());
		autoenv_paths.reverse();
		let Some(autoenv_path) = autoenv_paths.first() else {
			return Ok(());
		};
		let autoenv_path = *autoenv_path;

		// Get the package instance hash for this package.
		let package_identifier = package::Identifier::Path(autoenv_path.clone());
		let package_instance_hash = self
			.tg
			.clone()
			.create_package_instance(&package_identifier, false)
			.await?;

		// Run the operation.
		let function = Function {
			package_instance_hash,
			name: "env".into(),
		};
		let context = Self::create_default_context()?;
		let args = Vec::new();
		let operation = Operation::Call(Call {
			function,
			context,
			args,
		});
		let output = operation
			.run(&self.tg)
			.await
			.wrap_err("Failed to run the operation.")?;

		// Get the output artifact.
		let output_artifact_hash = output
			.into_artifact()
			.wrap_err("Expected the output to be an artifact.")?;

		// Check out the artifact.
		let artifact_path = self.tg.check_out_internal(output_artifact_hash).await?;

		// Get the path to the executable.
		let shell_activate_script_path = artifact_path.join("activate");

		// Print the source command.
		println!("source {}", shell_activate_script_path.to_str().unwrap());

		Ok(())
	}
}
