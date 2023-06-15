use super::Command;
use crate::{
	error::{error, return_error, Error, Result},
	instance::Instance,
	operation::Operation,
	system::System,
	template::Template,
	value::Value,
};
use futures::{future::try_join_all, FutureExt};
use itertools::Itertools;
use std::{
	collections::{BTreeMap, HashSet},
	path::Path,
	sync::Arc,
};

impl Command {
	#[tracing::instrument(skip(tg), ret)]
	pub async fn run(&self, tg: &Arc<Instance>) -> Result<Value> {
		let operation = Operation::Command(self.clone());
		operation.run(tg).await
	}

	pub(crate) async fn run_inner(&self, tg: &Arc<Instance>) -> Result<Value> {
		let permit = tg.process_semaphore.acquire().await;

		let result = if tg.options.sandbox_enabled {
			let system = self.system;
			match system {
				#[cfg(target_os = "linux")]
				System::Amd64Linux | System::Arm64Linux => self.run_inner_linux(tg).boxed(),

				#[cfg(target_os = "macos")]
				System::Amd64MacOs | System::Arm64MacOs => self.run_inner_macos(tg).boxed(),

				_ => return_error!(r#"This machine cannot run a process for system "{system}"."#),
			}
			.await
		} else {
			self.run_inner_basic(tg).await
		};

		drop(permit);
		result
	}

	pub(crate) fn render(
		&self,
		artifacts_directory_guest_path: &Path,
		output_guest_path: &Path,
	) -> Result<(String, BTreeMap<String, String>, Vec<String>)> {
		// Create a closure that renders a template.
		let render = |template: &Template| {
			template.render_sync(|component| match component {
				crate::template::Component::String(string) => Ok(string.into()),
				crate::template::Component::Artifact(artifact) => {
					Ok(artifacts_directory_guest_path
						.join(artifact.hash().to_string())
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

	pub(crate) async fn check_out_references(&self, tg: &Arc<Instance>) -> Result<()> {
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

		Ok(())
	}
}
