use super::server::Server;
use crate::{
	error::{return_error, Result, WrapErr},
	process::run::Path,
	system::System,
	temp::Temp,
	Instance,
};
use std::{
	collections::{BTreeMap, HashSet},
	sync::Arc,
};

impl Instance {
	pub async fn run_process_linux(
		self: &Arc<Self>,
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

		// Create the socket path.
		let socket_path = root_directory.path().join("socket");

		// Start the server.
		let server = Server::new(Arc::downgrade(self));
		let server_task = tokio::spawn({
			let socket_path = socket_path.clone();
			async move {
				server.serve(&socket_path).await.unwrap();
			}
		});

		// Create the command.
		let mut command = tokio::process::Command::new(&command);

		// Set the working directory.
		command.current_dir(&working_directory_path);

		// Set the envs.
		command.env_clear();
		command.envs(env);
		command.env("HOME", &home_directory_path);
		command.env("TANGRAM_SOCKET", &socket_path);

		// Set the args.
		command.args(args);

		// Spawn the child.
		let mut child = command.spawn().wrap_err("Failed to spawn the process.")?;

		// Wait for the child to exit.
		let status = child
			.wait()
			.await
			.wrap_err("Failed to wait for the process to exit.")?;

		// Stop the server.
		server_task.abort();
		server_task.await.ok();

		// Return an error if the process did not exit successfully.
		if !status.success() {
			return_error!("The process did not exit successfully.");
		}

		Ok(())
	}
}
