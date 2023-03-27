use crate::{
	error::{return_error, Result, WrapErr},
	system::System,
	temp::Temp,
	process::run::Path,
	Instance,
};
use std::collections::{BTreeMap, HashSet};

impl Instance {
	pub async fn run_process_linux(
		&self,
		_system: System,
		command: String,
		env: BTreeMap<String, String>,
		args: Vec<String>,
		_paths: HashSet<Path, fnv::FnvBuildHasher>,
		_network_enabled: bool,
	) -> Result<()> {
		// Create a temp path for the root directory.
		let root_directory = Temp::new(self);

		// Add the home directory to the root directory.
		let home_directory_path = root_directory.path().join("home").join("tangram");
		tokio::fs::create_dir_all(&home_directory_path).await?;

		// Add the working directory to the home directory.
		let working_directory_path = home_directory_path.join("work");
		tokio::fs::create_dir_all(&working_directory_path).await?;

		// Create the command.
		let mut command = tokio::process::Command::new(&command);

		// Set the working directory.
		command.current_dir(&working_directory_path);

		// Set the envs.
		command.env_clear();
		command.envs(env);
		command.env("HOME", &home_directory_path);

		// Set the args.
		command.args(args);

		// Spawn the child.
		let mut child = command.spawn().wrap_err("Failed to spawn the process.")?;

		// Wait for the child to exit.
		let status = child
			.wait()
			.await
			.wrap_err("Failed to wait for the process to exit.")?;

		// Return an error if the process did not exit successfully.
		if !status.success() {
			return_error!("The process did not exit successfully.");
		}

		Ok(())
	}
}
