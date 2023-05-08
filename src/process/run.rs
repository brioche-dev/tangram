use super::Process;
use crate::{
	artifact::Artifact,
	error::{error, return_error, Error, Result, WrapErr},
	instance::Instance,
	operation::Operation,
	system::System,
	temp::Temp,
	template::Template,
	util::fs,
	value::Value,
};
use futures::{future::try_join_all, FutureExt};
use std::{collections::HashSet, sync::Arc};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Path {
	pub kind: Kind,
	pub mode: Mode,
	pub host_path: fs::PathBuf,
	pub guest_path: fs::PathBuf,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Kind {
	File,
	Directory,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Mode {
	ReadOnly,
	ReadWrite,
}

impl Process {
	#[tracing::instrument(skip(tg), ret)]
	pub async fn run(&self, tg: &Arc<Instance>) -> Result<Value> {
		let operation = Operation::Process(self.clone());
		operation.run(tg).await
	}

	#[allow(clippy::too_many_lines)]
	pub(crate) async fn run_inner(&self, tg: &Arc<Instance>) -> Result<Value> {
		// Create a temp for the output.
		let output_temp = Temp::new(tg);
		tokio::fs::create_dir_all(output_temp.path())
			.await
			.wrap_err("Failed to create the directory for the output.")?;
		let output_temp_path = output_temp.path().join("output");

		// Get the system.
		let system = self.system;

		// Render the command template.
		let command = render(tg, &self.executable, &output_temp_path).await?;

		// Render the env templates.
		let env = try_join_all(self.env.iter().map(|(key, value)| {
			let output_temp_path = &output_temp_path;
			async move {
				let key = key.clone();
				let value = render(tg, value, output_temp_path).await?;
				Ok::<_, Error>((key, value))
			}
		}))
		.await?
		.into_iter()
		.collect();

		// Render the args templates.
		let args = try_join_all(
			self.args
				.iter()
				.map(|arg| render(tg, arg, &output_temp_path)),
		)
		.await?;

		// Collect the references.
		let mut references = HashSet::default();
		self.executable
			.collect_recursive_references(tg, &mut references)
			.await?;
		for value in self.env.values() {
			value
				.collect_recursive_references(tg, &mut references)
				.await?;
		}
		for arg in &self.args {
			arg.collect_recursive_references(tg, &mut references)
				.await?;
		}

		// Check out the references and collect the paths.
		let mut paths: HashSet<Path, fnv::FnvBuildHasher> =
			try_join_all(references.into_iter().map(|artifact| async move {
				let path = artifact.check_out_internal(tg).await?;
				let kind = match artifact {
					Artifact::File(_) | Artifact::Symlink(_) => Kind::File,
					Artifact::Directory(_) => Kind::Directory,
				};
				let path = Path {
					host_path: path.clone(),
					guest_path: path,
					mode: Mode::ReadOnly,
					kind,
				};
				Ok::<_, Error>(path)
			}))
			.await?
			.into_iter()
			.collect();

		// Add the output temp to the paths.
		paths.insert(Path {
			host_path: output_temp.path().to_owned(),
			guest_path: output_temp.path().to_owned(),
			mode: Mode::ReadWrite,
			kind: Kind::Directory,
		});

		// Enable unsafe options if a checksum was provided or if the unsafe flag was set.
		let enable_unsafe = self.checksum.is_some() || self.unsafe_;

		// Verify the safety constraints.
		if !enable_unsafe {
			if self.network {
				return_error!("Network access is not allowed in safe processes.");
			}
			if !self.host_paths.is_empty() {
				return_error!("Host paths are not allowed in safe processes.");
			}
		}

		// Handle the network flag.
		let network_enabled = self.network;

		// Handle the host paths.
		for host_path in &self.host_paths {
			// Determine the path kind.
			let metadata = tokio::fs::metadata(host_path)
				.await
				.wrap_err_with(|| format!("Failed to get metadata for host path {host_path:?}."))?;
			let kind = if metadata.is_dir() {
				Kind::Directory
			} else {
				Kind::File
			};

			// Insert the path.
			paths.insert(Path {
				host_path: host_path.into(),
				guest_path: host_path.into(),
				mode: Mode::ReadOnly,
				kind,
			});
		}

		// Run the process.
		match system {
			System::Amd64Linux | System::Arm64Linux => {
				#[cfg(target_os = "linux")]
				{
					Self::run_linux(tg, system, command, env, args, paths, network_enabled).boxed()
				}
				#[cfg(not(target_os = "linux"))]
				{
					return_error!("A Linux process cannot run on a non-Linux host.");
				}
			},
			System::Amd64Macos | System::Arm64Macos => {
				#[cfg(target_os = "macos")]
				{
					Self::run_macos(tg, system, command, env, args, paths, network_enabled).boxed()
				}
				#[cfg(not(target_os = "macos"))]
				{
					return_error!("A macOS process cannot run on a non-macOS host.");
				}
			},
		}
		.await?;

		tracing::debug!(output_temp_path = ?output_temp.path(), "Checking in the process output.");

		// Check in the output temp.
		let artifact = Artifact::check_in(tg, &output_temp_path)
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
					r#"The checksum did not match. Expected "{expected:?}" but got "{actual:?}"."#
				);
			}

			tracing::debug!("Validated the checksum.");
		}

		// Create the output.
		let value = Value::Artifact(artifact);

		Ok(value)
	}
}

/// Render a template for a process.
async fn render(tg: &Instance, template: &Template, output_path: &fs::Path) -> Result<String> {
	template
		.render(|component| async move {
			match component {
				crate::template::Component::String(string) => Ok(string.into()),
				crate::template::Component::Artifact(artifact) => Ok(tg
					.artifact_path(artifact.hash())
					.into_os_string()
					.into_string()
					.unwrap()
					.into()),
				crate::template::Component::Placeholder(placeholder) => {
					if placeholder.name == "output" {
						Ok(output_path.as_os_str().to_str().unwrap().into())
					} else {
						Err(error!(r#"Invalid placeholder "{}"."#, placeholder.name))
					}
				},
			}
		})
		.await
}
