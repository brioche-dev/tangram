use crate::{
	expression::{self, Artifact, Expression, UnixProcessOutput},
	hash::Hash,
	server::{temp::Temp, Evaluator, Server},
	system::System,
};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use async_trait::async_trait;
use futures::{
	future::{try_join3, try_join_all},
	FutureExt,
};
use std::{collections::BTreeMap, num::NonZeroUsize, sync::Arc};

mod js;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

pub struct Process {
	local_pool_handle: tokio_util::task::LocalPoolHandle,
}

impl Process {
	#[must_use]
	pub fn new() -> Process {
		let available_parallelism = std::thread::available_parallelism()
			.unwrap_or_else(|_| NonZeroUsize::new(1).unwrap())
			.into();
		let local_pool_handle = tokio_util::task::LocalPoolHandle::new(available_parallelism);
		Process { local_pool_handle }
	}
}

impl Default for Process {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl Evaluator for Process {
	async fn evaluate(
		&self,
		server: &Arc<Server>,
		hash: Hash,
		expression: &Expression,
	) -> Result<Option<Hash>> {
		let process = if let Expression::Process(process) = expression {
			process
		} else {
			return Ok(None);
		};
		let output = match process {
			expression::Process::Amd64Linux(process) => self
				.evaluate_unix_process(server, System::Amd64Linux, hash, process)
				.boxed(),
			expression::Process::Amd64Macos(process) => self
				.evaluate_unix_process(server, System::Amd64Macos, hash, process)
				.boxed(),
			expression::Process::Arm64Linux(process) => self
				.evaluate_unix_process(server, System::Arm64Linux, hash, process)
				.boxed(),
			expression::Process::Arm64Macos(process) => self
				.evaluate_unix_process(server, System::Arm64Macos, hash, process)
				.boxed(),
			expression::Process::Js(process) => {
				self.evaluate_js_process(server, hash, process).boxed()
			},
		}
		.await?;
		Ok(Some(output))
	}
}

impl Process {
	#[allow(clippy::too_many_lines)]
	pub async fn evaluate_unix_process(
		&self,
		server: &Arc<Server>,
		system: System,
		hash: Hash,
		process: &expression::UnixProcess,
	) -> Result<Hash> {
		// Evaluate the envs, command, and args.
		let (envs, command, args) = try_join3(
			server.evaluate(process.env, hash),
			server.evaluate(process.command, hash),
			server.evaluate(process.args, hash),
		)
		.await?;

		// Evaluate the outputs.
		let outputs: BTreeMap<String, UnixProcessOutput> =
			try_join_all(process.outputs.iter().map(|(key, output)| async {
				let dependencies =
					try_join_all(output.dependencies.iter().map(|(path, dependency)| async {
						let dependency = server.evaluate(*dependency, hash).await?;
						Ok::<_, anyhow::Error>((path.clone(), dependency))
					}))
					.await?
					.into_iter()
					.collect();
				let output = UnixProcessOutput { dependencies };
				Ok::<_, anyhow::Error>((key.clone(), output))
			}))
			.await?
			.into_iter()
			.collect();

		// Resolve the envs.
		let envs = match server.get_expression(envs).await? {
			Expression::Map(envs) => envs,
			_ => bail!(r#"Argument "envs" must evaluate to a map."#),
		};
		let mut envs: BTreeMap<String, String> =
			try_join_all(envs.iter().map(|(key, value)| async move {
				let key = key.as_ref().to_owned();
				let value = server.get_expression(*value).await?;
				let value = self.resolve(server, &value).await?;
				Ok::<_, anyhow::Error>((key, value))
			}))
			.await?
			.into_iter()
			.collect();

		// Create the temps for the outputs, add their dependencies, and their paths to the envs.
		let temps: BTreeMap<String, Temp> =
			try_join_all(outputs.iter().map(|(name, output)| async {
				let mut temp = server.create_temp().await?;
				for (path, dependency) in &output.dependencies {
					server
						.temp_add_dependency(&mut temp, path, *dependency)
						.await?;
				}
				Ok::<_, anyhow::Error>((name.clone(), temp))
			}))
			.await?
			.into_iter()
			.collect();
		envs.extend(
			temps
				.iter()
				.map(|(name, temp)| (name.clone(), server.temp_path(temp).display().to_string())),
		);

		// Resolve the command.
		let command = match server.get_expression(command).await? {
			Expression::Path(path) => path,
			_ => bail!("Command must evaluate to a path."),
		};
		let command_fragment = server
			.create_fragment(command.artifact)
			.await
			.context("Failed to create the fragment for the command.")?;
		let command_path = server.fragment_path(&command_fragment);
		let command = if let Some(path) = &command.path {
			command_path.join(path)
		} else {
			command_path
		};

		// Resolve the args.
		let args = match server.get_expression(args).await? {
			Expression::Array(array) => array,
			_ => bail!("Args must evaluate to an array."),
		};
		let args: Vec<String> = try_join_all(args.iter().copied().map(|value| async move {
			let value = server.get_expression(value).await?;
			let value = self.resolve(server, &value).await?;
			Ok::<_, anyhow::Error>(value)
		}))
		.await
		.context("Failed to resolve the args.")?;

		// Run the process.

		#[cfg(target_os = "linux")]
		self.run_linux_process(server, system, envs, command, args, hash)
			.await
			.context("Failed to run the process.")?;

		#[cfg(target_os = "macos")]
		self.run_macos_process(server, system, envs, command, args)
			.await
			.context("Failed to run the process.")?;

		// Checkin the temps.
		let artifacts: BTreeMap<String, Hash> =
			try_join_all(temps.into_iter().map(|(name, temp)| async {
				let artifact = server.checkin_temp(temp).await?;
				Ok::<_, anyhow::Error>((name, artifact))
			}))
			.await?
			.into_iter()
			.collect();

		// Create the output.
		let output_hash = if artifacts.len() == 1 {
			let hash = artifacts.into_values().next().unwrap();
			server
				.add_expression(&Expression::Artifact(Artifact { hash }))
				.await?
		} else {
			server
				.add_expression(&Expression::Map(
					try_join_all(artifacts.into_iter().map(|(name, hash)| async move {
						let artifact = server
							.add_expression(&Expression::Artifact(Artifact { hash }))
							.await?;
						Ok::<_, anyhow::Error>((name.into(), artifact))
					}))
					.await?
					.into_iter()
					.collect(),
				))
				.await?
		};

		Ok(output_hash)
	}

	#[async_recursion]
	async fn resolve(&self, server: &Arc<Server>, expression: &Expression) -> Result<String> {
		match expression {
			Expression::String(string) => Ok(string.as_ref().to_owned()),
			Expression::Template(template) => {
				let components = try_join_all(template.components.iter().copied().map(
					|component| async move {
						let component = server.get_expression(component).await?;
						let component = self.resolve(server, &component).await?;
						Ok::<_, anyhow::Error>(component)
					},
				))
				.await?;
				let string = components.join("");
				Ok(string)
			},
			Expression::Path(path) => {
				let fragment = server.create_fragment(path.artifact).await?;
				let fragment_path = server.fragment_path(&fragment);
				let fragment_path = if let Some(path) = &path.path {
					fragment_path.join(path)
				} else {
					fragment_path
				};
				let fragment_path_string = fragment_path.to_str().unwrap().to_owned();
				Ok(fragment_path_string)
			},
			_ => bail!("The expression to resolve must be a string, template, or path."),
		}
	}
}
