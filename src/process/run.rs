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
		let output_temp_path_host = output_temp.path().join("output");
		let output_temp_path_guest = if cfg!(target_os = "macos") {
			output_temp_path_host.clone()
		} else if cfg!(target_os = "linux") {
			let stripped = output_temp_path_host.strip_prefix(tg.path()).map_err(|_| {
				error!("Unable to strip Tangram directory from process output temp directory")
			})?;
			fs::PathBuf::from("/.tangram").join(stripped)
		} else {
			unreachable!()
		};

		// Get the system.
		let system = self.system;

		// Get the path to the artifacts directory, as visible by the guest.
		let artifacts_directory = if cfg!(target_os = "macos") {
			tg.artifacts_path()
		} else if cfg!(target_os = "linux") {
			"/.tangram/artifacts".into()
		} else {
			unreachable!()
		};

		// Render the command template.
		let command = render(
			&self.executable,
			&artifacts_directory,
			&output_temp_path_guest,
		)
		.await?;

		// Render the env templates.
		let mut env: std::collections::BTreeMap<String, String> =
			try_join_all(self.env.iter().map(|(key, value)| {
				let artifacts_directory = &artifacts_directory;
				let output_temp_path = &output_temp_path_guest;
				async move {
					let key = key.clone();
					let value = render(value, artifacts_directory, output_temp_path).await?;
					Ok::<_, Error>((key, value))
				}
			}))
			.await?
			.into_iter()
			.collect();

		// Set `TG_PLACEHOLDER_OUTPUT`.
		env.insert(
			"TANGRAM_PLACEHOLDER_OUTPUT".to_string(),
			output_temp_path_guest.display().to_string(),
		);

		// Render the args templates.
		let args = try_join_all(
			self.args
				.iter()
				.map(|arg| render(arg, &artifacts_directory, &output_temp_path_guest)),
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

		// Check out the references.
		try_join_all(references.into_iter().map(|artifact| async move {
			artifact.check_out_internal(tg).await?;
			Ok::<_, Error>(())
		}))
		.await?;

		let mut paths: HashSet<Path, fnv::FnvBuildHasher> = HashSet::default();

		// Add the output temp to the paths.
		paths.insert(Path {
			host_path: output_temp_path_host.parent().unwrap().to_owned(),
			guest_path: output_temp_path_guest.parent().unwrap().to_owned(),
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
			// Check if any existing mount point is a parent of this path and skip adding it.
			// Note: This operation is potentially expensive if there are many host paths. However, the assumption is that there are only a handful and this loop will only iterate a few times.
			let parent = paths.iter().find(|existing| {
				std::path::PathBuf::from(host_path).starts_with(&existing.host_path)
			});
			if parent.is_some() {
				continue;
			}

			// Check if this path is a parent of any mounts that have already been added, and if so remove the child.
			let child = paths
				.iter()
				.find(|existing| existing.host_path.starts_with(host_path));
			if let Some(child) = child.cloned() {
				paths.remove(&child);
			}

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

		tracing::debug!(?output_temp_path_host, "Checking in the process output.");

		// Check in the output temp.
		let artifact = Artifact::check_in(tg, &output_temp_path_host)
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
async fn render(
	template: &Template,
	artifacts_directory: &std::path::Path,
	output_path: &fs::Path,
) -> Result<String> {
	template
		.render(|component| async move {
			match component {
				crate::template::Component::String(string) => Ok(string.into()),
				crate::template::Component::Artifact(artifact) => Ok(artifacts_directory
					.join(artifact.hash().to_string())
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
