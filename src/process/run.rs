use super::Process;
use crate::{
	error::{return_error, Error, Result, WrapErr},
	system::System,
	temp::Temp,
	template::Path,
	value::Value,
	Instance,
};
use futures::{future::try_join_all, FutureExt};
use std::{collections::HashSet, sync::Arc};

impl Instance {
	#[allow(clippy::too_many_lines)]
	pub async fn run_process(self: &Arc<Self>, process: &Process) -> Result<Value> {
		// Create a temp for the output.
		let output_temp = Temp::new(self);

		// Create the placeholder values for rendering templates.
		let placeholder_values = [(
			"output".to_owned(),
			Path {
				host_path: output_temp.path().to_owned(),
				guest_path: output_temp.path().to_owned(),
				read: true,
				write: true,
				create: true,
			},
		)]
		.into();

		// Render the env templates.
		let env = {
			let placeholder_values = &placeholder_values;
			try_join_all(process.env.iter().map({
				|(key, value)| async move {
					let value = self.render(value, placeholder_values).await?;
					Ok::<_, Error>((key, value))
				}
			}))
			.await?
		};

		// Render the command template.
		let command = self.render(&process.command, &placeholder_values).await?;

		// Render the args templates.
		let args = {
			let placeholder_values = &placeholder_values;
			try_join_all(process.args.iter().map({
				|value| async move {
					let value = self.render(value, placeholder_values).await?;
					Ok::<_, Error>(value)
				}
			}))
			.await?
		};

		// Collect the paths and get the strings for the env, command, and args.
		let mut paths = HashSet::default();
		let env = env
			.into_iter()
			.map(|(key, value)| {
				paths.extend(value.paths);
				(key.to_string(), value.string)
			})
			.collect();
		paths.extend(command.paths);
		let command = command.string;
		let args = args
			.into_iter()
			.map(|value| {
				paths.extend(value.paths);
				value.string
			})
			.collect();

		// Enable networking if the process has a checksum or is unsafe.
		let network_enabled = process.checksum.is_some() || process.is_unsafe;

		// Run the process.
		match process.system {
			System::Amd64Linux | System::Arm64Linux => {
				#[cfg(target_os = "linux")]
				{
					self.run_process_linux(
						process.system,
						env,
						command,
						args,
						paths,
						network_enabled,
					)
					.boxed()
				}
				#[cfg(not(target_os = "linux"))]
				{
					return_error!("A Linux process cannot run on a non-Linux host.");
				}
			},
			System::Amd64Macos | System::Arm64Macos => {
				#[cfg(target_os = "macos")]
				{
					self.run_process_macos(
						process.system,
						env,
						command,
						args,
						paths,
						network_enabled,
					)
					.boxed()
				}
				#[cfg(not(target_os = "macos"))]
				{
					return_error!("A macOS process cannot run on a non-macOS host.");
				}
			},
		}
		.await?;

		// Check in the output temp.
		let output_hash = self
			.check_in(output_temp.path())
			.await
			.wrap_err("Failed to check in the output.")?;

		// Verify the checksum if one was provided.
		if let Some(expected) = process.checksum.clone() {
			let actual = self
				.compute_artifact_checksum(output_hash, expected.algorithm())
				.await
				.wrap_err("Failed to compute the checksum.")?;
			if expected != actual {
				return_error!(
					r#"The checksum did not match. Expected "{expected:?}" but got "{actual:?}"."#
				);
			}
		}

		// Create the output.
		let output = Value::Artifact(output_hash);

		Ok(output)
	}
}
