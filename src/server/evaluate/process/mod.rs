use crate::{
	expression::{self, Artifact, Expression, UnixProcessOutput},
	hash::Hash,
	server::{temp::Temp, Server},
	system::System,
};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use futures::{
	future::{try_join3, try_join_all},
	FutureExt,
};
use std::{collections::BTreeMap, sync::Arc};

mod js;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

impl Server {
	pub async fn evaluate_process(
		self: &Arc<Self>,
		process: &expression::Process,
		parent_hash: Hash,
	) -> Result<Hash> {
		match process {
			expression::Process::Amd64Linux(process) => self
				.evaluate_unix_process(System::Amd64Linux, process, parent_hash)
				.boxed(),
			expression::Process::Amd64Macos(process) => self
				.evaluate_unix_process(System::Amd64Macos, process, parent_hash)
				.boxed(),
			expression::Process::Arm64Linux(process) => self
				.evaluate_unix_process(System::Arm64Linux, process, parent_hash)
				.boxed(),
			expression::Process::Arm64Macos(process) => self
				.evaluate_unix_process(System::Arm64Macos, process, parent_hash)
				.boxed(),
			expression::Process::Js(process) => {
				self.evaluate_js_process(process, parent_hash).boxed()
			},
		}
		.await
	}

	#[allow(clippy::too_many_lines)]
	pub async fn evaluate_unix_process(
		self: &Arc<Self>,
		system: System,
		process: &expression::UnixProcess,
		parent_hash: Hash,
	) -> Result<Hash> {
		// Evaluate the envs, command, and args.
		let (envs, command, args) = try_join3(
			self.evaluate(process.env, parent_hash),
			self.evaluate(process.command, parent_hash),
			self.evaluate(process.args, parent_hash),
		)
		.await?;

		// Evaluate the outputs.
		let outputs: BTreeMap<String, UnixProcessOutput> =
			try_join_all(process.outputs.iter().map(|(key, output)| async {
				let dependencies =
					try_join_all(output.dependencies.iter().map(|(path, dependency)| async {
						let dependency = self.evaluate(*dependency, parent_hash).await?;
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
		let envs = match self.get_expression(envs).await? {
			Expression::Map(envs) => envs,
			_ => bail!(r#"Argument "envs" must evaluate to a map."#),
		};
		let mut envs: BTreeMap<String, String> =
			try_join_all(envs.iter().map(|(key, value)| async move {
				let key = key.as_ref().to_owned();
				let value = self.get_expression(*value).await?;
				let value = self.resolve(&value).await?;
				Ok::<_, anyhow::Error>((key, value))
			}))
			.await?
			.into_iter()
			.collect();

		// Create the temps for the outputs, add their dependencies, and their paths to the envs.
		let temps: BTreeMap<String, Temp> =
			try_join_all(outputs.iter().map(|(name, output)| async {
				let mut temp = self.create_temp().await?;
				for (path, dependency) in &output.dependencies {
					let dependency = self.get_expression(*dependency).await?;
					let artifact = match dependency {
						Expression::Artifact(artifact) => artifact,
						_ => bail!(r#"Dependency must evaluate to an artifact."#),
					};
					self.temp_add_dependency(&mut temp, path, artifact).await?;
				}
				Ok((name.clone(), temp))
			}))
			.await?
			.into_iter()
			.collect();
		envs.extend(
			temps
				.iter()
				.map(|(name, temp)| (name.clone(), self.temp_path(temp).display().to_string())),
		);

		// Resolve the command.
		let command = match self.get_expression(command).await? {
			Expression::Path(path) => path,
			_ => bail!("Command must evaluate to a path."),
		};
		let command_artifact = match self.get_expression(command.artifact).await? {
			Expression::Artifact(artifact) => artifact,
			_ => bail!("Command artifact must evaluate to an artifact."),
		};
		let command_fragment = self
			.create_fragment(command_artifact)
			.await
			.context("Failed to create the fragment for the command.")?;
		let command_path = self.fragment_path(&command_fragment);
		let command = if let Some(path) = &command.path {
			command_path.join(path.as_ref())
		} else {
			command_path
		};

		// Resolve the args.
		let args = match self.get_expression(args).await? {
			Expression::Array(array) => array,
			_ => bail!("Args must evaluate to an array."),
		};
		let args: Vec<String> = try_join_all(args.iter().copied().map(|value| async move {
			let value = self.get_expression(value).await?;
			let value = self.resolve(&value).await?;
			Ok::<_, anyhow::Error>(value)
		}))
		.await
		.context("Failed to resolve the args.")?;

		// Run the process.

		#[cfg(target_os = "linux")]
		self.run_linux_process(system, envs, command, args, parent_hash)
			.await
			.context("Failed to run the process.")?;

		#[cfg(target_os = "macos")]
		self.run_macos_process(system, envs, command, args)
			.await
			.context("Failed to run the process.")?;

		// Checkin the temps.
		let artifacts: BTreeMap<String, Artifact> =
			try_join_all(temps.into_iter().map(|(name, temp)| async {
				let artifact = self.checkin_temp(temp).await?;
				Ok::<_, anyhow::Error>((name, artifact))
			}))
			.await?
			.into_iter()
			.collect();

		// Create the output.
		let output_hash = if artifacts.len() == 1 {
			self.add_expression(&Expression::Artifact(
				artifacts.into_values().next().unwrap(),
			))
			.await?
		} else {
			self.add_expression(&Expression::Map(
				try_join_all(artifacts.into_iter().map(|(name, artifact)| async move {
					Ok::<_, anyhow::Error>((
						name.into(),
						self.add_expression(&Expression::Artifact(artifact)).await?,
					))
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
	async fn resolve(self: &Arc<Self>, expression: &Expression) -> Result<String> {
		match expression {
			Expression::String(string) => Ok(string.as_ref().to_owned()),
			Expression::Template(template) => {
				let components = try_join_all(template.components.iter().copied().map(
					|component| async move {
						let component = self.get_expression(component).await?;
						let component = self.resolve(&component).await?;
						Ok::<_, anyhow::Error>(component)
					},
				))
				.await?;
				let string = components.join("");
				Ok(string)
			},
			Expression::Path(path) => {
				let artifact = match self.get_expression(path.artifact).await? {
					Expression::Artifact(artifact) => artifact,
					_ => bail!("Expected artifact."),
				};
				let fragment = self.create_fragment(artifact).await?;
				let fragment_path = self.fragment_path(&fragment);
				let fragment_path = if let Some(path) = &path.path {
					fragment_path.join(path.as_ref())
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
