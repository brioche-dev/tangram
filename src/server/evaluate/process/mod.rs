use crate::{
	artifact::Artifact,
	expression,
	server::{temp::Temp, Server},
	value::Value,
};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use futures::future::try_join_all;
use std::{collections::BTreeMap, sync::Arc};

mod js;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

impl Server {
	pub async fn evaluate_process(self: &Arc<Self>, process: expression::Process) -> Result<Value> {
		match process {
			expression::Process::Amd64Linux(process)
			| expression::Process::Amd64Macos(process)
			| expression::Process::Arm64Linux(process)
			| expression::Process::Arm64Macos(process) => self.evaluate_unix_process(process).await,
			expression::Process::Js(process) => self.evaluate_js_process(process).await,
		}
	}

	pub async fn evaluate_unix_process(
		self: &Arc<Self>,
		process: expression::UnixProcess,
	) -> Result<Value> {
		let crate::expression::UnixProcess { command, args, .. } = process;

		// Create the temps for the outputs and add their dependencies.
		let temps: BTreeMap<String, Temp> =
			try_join_all(process.outputs.into_iter().map(|(name, output)| {
				async {
					let mut temp = self.create_temp().await?;
					for (path, dependency) in output.dependencies {
						let dependency = self.evaluate(*dependency).await?;
						let artifact = match dependency {
							Value::Artifact(artifact) => artifact,
							_ => bail!(r#"Dependency must evaluate to an artifact."#),
						};
						self.add_dependency(&mut temp, &path, artifact).await?;
					}
					Ok((name, temp))
				}
			}))
			.await?
			.into_iter()
			.collect();

		// Evaluate the envs.
		let envs = self.evaluate(*process.env).await?;
		let envs = match envs {
			Value::Map(envs) => envs,
			_ => bail!(r#"Argument "envs" must evaluate to a map."#),
		};
		let mut envs: BTreeMap<String, String> =
			futures::future::try_join_all(envs.into_iter().map(|(key, value)| {
				async { Ok::<_, anyhow::Error>((key, self.resolve(value).await?)) }
			}))
			.await?
			.into_iter()
			.collect();

		// Add the paths to the temps to the envs.
		envs.extend(
			temps
				.iter()
				.map(|(name, temp)| (name.clone(), self.temp_path(temp).display().to_string())),
		);

		// Evaluate the command.
		let command = self.evaluate(*command).await?;
		let command = match command {
			Value::Path(path) => path,
			_ => bail!("Command must be a path."),
		};
		let command_fragment = self.create_fragment(command.artifact).await?;
		let command_path = self.fragment_path(&command_fragment);
		let command = if let Some(path) = &command.path {
			command_path.join(path)
		} else {
			command_path
		};

		// Evaluate the args.
		let args = self.evaluate(*args).await?;
		let args = match args {
			Value::Array(array) => array,
			_ => bail!("Args must be an array."),
		};
		let args: Vec<String> =
			futures::future::try_join_all(args.into_iter().map(|value| self.resolve(value)))
				.await?;

		// Run the process.

		#[cfg(target_os = "linux")]
		self.run_linux_process(envs, command, args)
			.await
			.context("Failed to run the process.")?;

		#[cfg(target_os = "macos")]
		self.run_macos_process(envs, command, args)
			.await
			.context("Failed to run the process.")?;

		// Checkin the temps.
		let artifacts: BTreeMap<String, Artifact> =
			try_join_all(temps.into_iter().map(|(name, temp)| {
				async {
					let artifact = self.checkin_temp(temp).await?;
					Ok::<_, anyhow::Error>((name, artifact))
				}
			}))
			.await?
			.into_iter()
			.collect();

		// Create the output value.
		let value = if artifacts.len() == 1 {
			Value::Artifact(artifacts.into_values().next().unwrap())
		} else {
			Value::Map(
				artifacts
					.into_iter()
					.map(|(name, artifact)| (name, Value::Artifact(artifact)))
					.collect(),
			)
		};

		Ok(value)
	}

	#[async_recursion]
	async fn resolve(self: &Arc<Self>, value: Value) -> Result<String> {
		match value {
			Value::String(string) => Ok(string),
			Value::Template(template) => {
				let components = try_join_all(
					template
						.components
						.into_iter()
						.map(|value| self.resolve(value)),
				)
				.await?;
				let string = components.join("");
				Ok(string)
			},
			Value::Path(path) => {
				let fragment = self.create_fragment(path.artifact).await?;
				let fragment_path = self.fragment_path(&fragment);
				let fragment_path = if let Some(path) = path.path {
					fragment_path.join(path)
				} else {
					fragment_path
				};
				let fragment_path_string = fragment_path.to_str().unwrap().to_owned();
				Ok(fragment_path_string)
			},
			_ => bail!("The value to resolve must be a string, template, or path."),
		}
	}
}
