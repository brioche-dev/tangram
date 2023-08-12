use super::Task;
use crate::{
	error::{error, return_error, Error, Result},
	instance::Instance,
	operation::Operation,
	system::System,
	template::Template,
	value::Value,
};
use futures::{stream::FuturesUnordered, FutureExt, TryStreamExt};
use itertools::Itertools;
use std::{
	collections::{BTreeMap, HashSet},
	path::Path,
};

impl Task {
	#[tracing::instrument(skip(tg), ret)]
	pub async fn run(&self, tg: &Instance) -> Result<Value> {
		let operation = Operation::Task(self.clone());
		operation.evaluate(tg, None).await
	}

	pub(crate) async fn run_inner(&self, tg: &Instance) -> Result<Value> {
		let _permit = tg.command_semaphore.acquire().await;
		if tg.options.sandbox_enabled {
			let host = self.host;
			match host {
				#[cfg(target_os = "linux")]
				System::Amd64Linux | System::Arm64Linux => self.run_inner_linux(tg).boxed(),

				#[cfg(target_os = "macos")]
				System::Amd64MacOs | System::Arm64MacOs => self.run_inner_macos(tg).boxed(),

				_ => return_error!(r#"This machine cannot run a process for host "{host}"."#),
			}
			.await
		} else {
			self.run_inner_basic(tg).await
		}
	}

	pub(crate) fn render(
		&self,
		artifacts_directory_guest_path: &Path,
		output_guest_path: &Path,
	) -> Result<(String, BTreeMap<String, String>, Vec<String>)> {
		// Create a closure that renders a template.
		let render = |template: &Template| {
			template.try_render_sync(|component| match component {
				crate::template::Component::String(string) => Ok(string.into()),
				crate::template::Component::Artifact(artifact) => {
					Ok(artifacts_directory_guest_path
						.join(artifact.id().to_string())
						.into_os_string()
						.into_string()
						.unwrap()
						.into())
				},
				crate::template::Component::Placeholder(placeholder) => {
					if placeholder.name == "output" {
						Ok(output_guest_path.as_os_str().to_str().unwrap().into())
					} else {
						Err(error!(r#"Invalid placeholder "{}"."#, placeholder.name))
					}
				},
			})
		};

		// Render the executable.
		let executable = render(&self.executable)?;

		// Render the env.
		let env: std::collections::BTreeMap<String, String> = self
			.env
			.iter()
			.map(|(key, value)| {
				let key = key.clone();
				let value = render(value)?;
				Ok::<_, Error>((key, value))
			})
			.try_collect()?;

		// Render the args.
		let args: Vec<String> = self.args.iter().map(render).try_collect()?;

		Ok((executable, env, args))
	}

	pub(crate) async fn check_out_references(&self, tg: &Instance) -> Result<()> {
		// Collect the references.
		let mut references = HashSet::<_, fnv::FnvBuildHasher>::default();
		references.extend(self.executable.artifacts().cloned());
		for value in self.env.values() {
			references.extend(value.artifacts().cloned());
		}
		for arg in &self.args {
			references.extend(arg.artifacts().cloned());
		}

		// Check out the references.
		references
			.into_iter()
			.map(|artifact| async move {
				artifact.check_out_internal(tg).await?;
				Ok::<_, Error>(())
			})
			.collect::<FuturesUnordered<_>>()
			.try_collect()
			.await?;

		Ok(())
	}
}
