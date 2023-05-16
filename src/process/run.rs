use super::{
	mount::{self, Mount},
	Process,
};
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
use itertools::Itertools;
use std::{collections::HashSet, sync::Arc};

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
		let output_temp_host_path = output_temp.path().to_owned();
		let output_temp_guest_path = if cfg!(target_os = "macos") {
			output_temp_host_path.clone()
		} else if cfg!(target_os = "linux") {
			fs::PathBuf::from(format!("/.tangram/temps/{}", output_temp.id()))
		} else {
			unreachable!()
		};

		// Create a directory for the output.
		let output_host_path = output_temp_host_path.join("output");
		let output_guest_path = output_temp_guest_path.join("output");
		tokio::fs::create_dir_all(&output_temp_host_path)
			.await
			.wrap_err("Failed to create the directory for the output.")?;

		// Get the artifacts guest path.
		let artifacts_guest_path = if cfg!(target_os = "macos") {
			tg.artifacts_path()
		} else if cfg!(target_os = "linux") {
			"/.tangram/artifacts".into()
		} else {
			unreachable!()
		};

		// Create a closure that renders a template.
		let render = |template: &Template| {
			template.render_sync(|component| match component {
				crate::template::Component::String(string) => Ok(string.into()),
				crate::template::Component::Artifact(artifact) => Ok(artifacts_guest_path
					.join(artifact.hash().to_string())
					.into_os_string()
					.into_string()
					.unwrap()
					.into()),
				crate::template::Component::Placeholder(placeholder) => {
					if placeholder.name == "output" {
						Ok(output_guest_path.as_os_str().to_str().unwrap().into())
					} else {
						Err(error!(r#"Invalid placeholder "{}"."#, placeholder.name))
					}
				},
			})
		};

		// Get the system.
		let system = self.system;

		// Render the command template.
		let command = render(&self.executable)?;

		// Render the env.
		let mut env: std::collections::BTreeMap<String, String> = self
			.env
			.iter()
			.map(|(key, value)| {
				let key = key.clone();
				let value = render(value)?;
				Ok::<_, Error>((key, value))
			})
			.try_collect()?;

		// Set `TG_PLACEHOLDER_OUTPUT`.
		env.insert(
			"TANGRAM_PLACEHOLDER_OUTPUT".to_owned(),
			output_guest_path.to_str().unwrap().to_owned(),
		);

		// Render the args.
		let args: Vec<String> = self.args.iter().map(render).try_collect()?;

		// Collect the references.
		let mut references = HashSet::default();
		self.executable.collect_references(&mut references);
		for value in self.env.values() {
			value.collect_references(&mut references);
		}
		for arg in &self.args {
			arg.collect_references(&mut references);
		}

		// Check out the references.
		try_join_all(references.into_iter().map(|artifact| async move {
			artifact.check_out_internal(tg).await?;
			Ok::<_, Error>(())
		}))
		.await?;

		// Create the mounts.
		let mut mounts: HashSet<Mount, fnv::FnvBuildHasher> = HashSet::default();

		// Add the output temp path to the mounts.
		mounts.insert(Mount {
			host_path: output_temp_host_path.parent().unwrap().to_owned(),
			guest_path: output_temp_guest_path.parent().unwrap().to_owned(),
			mode: mount::Mode::ReadWrite,
			kind: mount::Kind::Directory,
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
			// If any existing mount's host path is a parent of this host path, then continue.
			let parent = mounts.iter().find(|existing| {
				std::path::PathBuf::from(host_path).starts_with(&existing.host_path)
			});
			if parent.is_some() {
				continue;
			}

			// If this host path is a parent of any existing mount's host path, then remove the existing mount.
			let child = mounts
				.iter()
				.find(|existing| existing.host_path.starts_with(host_path));
			if let Some(child) = child.cloned() {
				mounts.remove(&child);
			}

			// Determine the kind.
			let metadata = tokio::fs::metadata(host_path).await.wrap_err_with(|| {
				format!(r#"Failed to get the metadata for the host path "{host_path}"."#)
			})?;
			let kind = if metadata.is_dir() {
				mount::Kind::Directory
			} else {
				mount::Kind::File
			};

			// Insert the mount.
			mounts.insert(Mount {
				host_path: host_path.into(),
				guest_path: host_path.into(),
				mode: mount::Mode::ReadOnly,
				kind,
			});
		}

		// Run the process.
		match system {
			#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
			System::Amd64Linux => Self::run_linux(
				tg,
				artifacts_guest_path,
				system,
				command,
				env,
				args,
				mounts,
				network_enabled,
			)
			.boxed(),

			#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
			System::Arm64Linux => Self::run_linux(
				tg,
				artifacts_guest_path,
				system,
				command,
				env,
				args,
				mounts,
				network_enabled,
			)
			.boxed(),

			#[cfg(target_os = "macos")]
			System::Amd64MacOs | System::Arm64MacOs => {
				Self::run_macos(tg, system, command, env, args, mounts, network_enabled).boxed()
			},

			_ => return_error!(r#"This machine cannot run a process for system "{system}"."#),
		}
		.await?;

		tracing::debug!(?output_host_path, "Checking in the process output.");

		// Check in the output.
		let artifact = Artifact::check_in(tg, &output_host_path)
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
