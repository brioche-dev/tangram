use super::Process;
use crate::{
	error::{return_error, Error, Result, WrapErr},
	system::System,
	temp::Temp,
	template::Template,
	util::fs,
	value::Value,
	Instance,
};
use futures::{future::try_join_all, FutureExt};
use std::{collections::HashSet, sync::Arc};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Path {
	pub host_path: fs::PathBuf,
	pub guest_path: fs::PathBuf,
	pub mode: Mode,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Mode {
	Readonly,
	ReadWrite,
}

impl Instance {
	#[allow(clippy::too_many_lines)]
	#[tracing::instrument(skip(self), ret)]
	pub async fn run_process(self: &Arc<Self>, process: &Process) -> Result<Value> {
		// Create a temp for the output.
		let output_temp = Temp::new(self);
		let output_temp_path = output_temp.path();

		tracing::debug!(output_temp_path = ?output_temp.path(), "Prepareing to run process.");

		// Render the command template.
		let command = render(self, &process.command, output_temp_path).await?;

		// Render the env templates.
		let env = try_join_all(process.env.iter().map(|(key, value)| async move {
			let key = key.clone();
			let value = render(self, value, output_temp_path).await?;
			Ok::<_, Error>((key, value))
		}))
		.await?
		.into_iter()
		.collect();

		// Render the args templates.
		let args = try_join_all(
			process
				.args
				.iter()
				.map(|arg| render(self, arg, output_temp_path)),
		)
		.await?;

		// Collect the references.
		let mut references = Vec::new();
		process.command.collect_references(self, &mut references)?;
		for value in process.env.values() {
			value.collect_references(self, &mut references)?;
		}
		for arg in &process.args {
			arg.collect_references(self, &mut references)?;
		}

		// Check out the references and collect the paths
		let mut paths: HashSet<Path, fnv::FnvBuildHasher> =
			try_join_all(references.into_iter().map(|artifact_hash| async move {
				let path = self.check_out_internal(artifact_hash).await?;
				let path = Path {
					host_path: path.clone(),
					guest_path: path,
					mode: Mode::Readonly,
				};
				Ok::<_, Error>(path)
			}))
			.await?
			.into_iter()
			.collect();

		// Add the output path to the paths.
		paths.insert(Path {
			host_path: output_temp_path.to_owned(),
			guest_path: output_temp_path.to_owned(),
			mode: Mode::ReadWrite,
		});

		// Enable networking if the process has a checksum or is unsafe.
		let network_enabled = process.checksum.is_some() || process.is_unsafe;

		// Run the process.
		match process.system {
			System::Amd64Linux | System::Arm64Linux => {
				#[cfg(target_os = "linux")]
				{
					self.run_process_linux(
						process.system,
						command,
						env,
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
						command,
						env,
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

		tracing::debug!(output_temp_path = ?output_temp.path(), "Checking in the output.");

		// Check in the output temp.
		let output_hash = self
			.check_in(output_temp.path())
			.await
			.wrap_err("Failed to check in the output.")?;

		tracing::info!(output_hash = ?output_hash, "Checked in process output.");

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

			tracing::debug!("Validated checksum");
		}

		// Create the output.
		let output = Value::Artifact(output_hash);

		Ok(output)
	}
}

async fn render(tg: &Instance, template: &Template, output_path: &fs::Path) -> Result<String> {
	template
		.render(|component| async move {
			match component {
				crate::template::Component::String(string) => Ok(string.into()),
				crate::template::Component::Artifact(artifact_hash) => Ok(tg
					.checkout_path(*artifact_hash)
					.into_os_string()
					.into_string()
					.unwrap()
					.into()),
				crate::template::Component::Placeholder(placeholder) => {
					if placeholder.name != "output" {
						return_error!(r#"Invalid placeholder "{}"."#, placeholder.name);
					}
					Ok(output_path.as_os_str().to_str().unwrap().into())
				},
			}
		})
		.await
}
