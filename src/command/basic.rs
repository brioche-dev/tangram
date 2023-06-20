use super::Command;
use crate::{
	artifact::{self, Artifact},
	error::{return_error, Result, WrapErr},
	instance::Instance,
	temp::Temp,
	value::Value,
};
use std::sync::Arc;

impl Command {
	pub async fn run_inner_basic(&self, tg: &Arc<Instance>) -> Result<Value> {
		// Check out the references.
		self.check_out_references(tg)
			.await
			.wrap_err("Failed to check out the references.")?;

		// Create a temp for the root.
		let root_temp = Temp::new(tg);
		let root_path = root_temp.path().to_owned();
		tokio::fs::create_dir_all(&root_path)
			.await
			.wrap_err("Failed to create the root directory.")?;

		// Create a temp for the output.
		let output_temp = Temp::new(tg);

		// Create the output parent directory.
		let output_parent_directory_path = output_temp.path().to_owned();
		tokio::fs::create_dir_all(&output_parent_directory_path)
			.await
			.wrap_err("Failed to create the output parent directory.")?;

		// Create the output path.
		let output_path = output_parent_directory_path.join("output");

		// Get the path for the artifacts directory.
		let tangram_directory_path = tg.path();

		// Create the home directory.
		let home_directory_path = root_path.join("Users/tangram");
		tokio::fs::create_dir_all(&home_directory_path)
			.await
			.wrap_err("Failed to create the home directory.")?;

		// Create the working directory.
		let working_directory_path = root_path.join("Users/tangram/work");
		tokio::fs::create_dir_all(&working_directory_path)
			.await
			.wrap_err("Failed to create the working directory.")?;

		// Render the executable, env, and args.
		let (executable, mut env, args) = self.render(&tg.artifacts_path(), &output_path)?;

		// Enable unsafe options if a checksum was provided or if the unsafe flag was set.
		let enable_unsafe = self.checksum.is_some() || self.unsafe_;

		// Verify the safety constraints.
		if !enable_unsafe && self.network {
			return_error!("Network access is not allowed in safe processes.");
		}

		// Create the socket path.
		let socket_path = root_path.join("socket");

		// Set `$HOME`.
		env.insert(
			"HOME".to_owned(),
			home_directory_path.to_str().unwrap().to_owned(),
		);

		// Set `$TANGRAM_PATH`.
		env.insert(
			"TANGRAM_PATH".to_owned(),
			tangram_directory_path.to_str().unwrap().to_owned(),
		);

		// Set `$TG_PLACEHOLDER_OUTPUT`.
		env.insert(
			"TANGRAM_PLACEHOLDER_OUTPUT".to_owned(),
			output_path.to_str().unwrap().to_owned(),
		);

		// Set `$TANGRAM_SOCKET`.
		env.insert(
			"TANGRAM_SOCKET".to_owned(),
			socket_path.to_str().unwrap().to_owned(),
		);

		// Create the command.
		let mut command = tokio::process::Command::new(&executable);

		// Set the working directory.
		command.current_dir(&working_directory_path);

		// Set the envs.
		command.env_clear();
		command.envs(env);

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

		tracing::debug!(?output_path, "Checking in the process output.");

		// Create the output.
		let value = if tokio::fs::try_exists(&output_path).await? {
			// Check in the output.
			let options = artifact::checkin::Options {
				artifacts_paths: vec![tg.artifacts_path()],
			};
			let artifact = Artifact::check_in_with_options(tg, &output_path, &options)
				.await
				.wrap_err("Failed to check in the output.")?;

			tracing::info!(?artifact, "Checked in the process output.");

			// Verify the checksum if one was provided.
			if let Some(expected) = self.checksum.clone() {
				let actual = artifact
					.checksum(tg, expected.algorithm())
					.await
					.wrap_err("Failed to compute the checksum.")?;
				if expected != actual {
					return_error!(
						r#"The checksum did not match. Expected "{expected}" but got "{actual}"."#
					);
				}

				tracing::debug!("Validated the checksum.");
			}
			Value::Artifact(artifact)
		} else {
			Value::Null
		};

		Ok(value)
	}
}
