use crate::{
	artifact::Artifact,
	expression::{self, Expression},
	server::{temp::Temp, Server},
	system::System,
};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use futures::{future::try_join_all, FutureExt};
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
	) -> Result<Expression> {
		match process {
			expression::Process::Amd64Linux(process) => self
				.evaluate_unix_process(System::Amd64Linux, process)
				.boxed(),
			expression::Process::Amd64Macos(process) => self
				.evaluate_unix_process(System::Amd64Macos, process)
				.boxed(),
			expression::Process::Arm64Linux(process) => self
				.evaluate_unix_process(System::Arm64Linux, process)
				.boxed(),
			expression::Process::Arm64Macos(process) => self
				.evaluate_unix_process(System::Arm64Macos, process)
				.boxed(),
			expression::Process::Js(process) => self.evaluate_js_process(process).boxed(),
		}
		.await
	}

	pub async fn evaluate_unix_process(
		self: &Arc<Self>,
		system: System,
		process: &expression::UnixProcess,
	) -> Result<Expression> {
		// Create the temps for the outputs and add their dependencies.
		let temps: BTreeMap<String, Temp> =
			try_join_all(process.outputs.iter().map(|(name, output)| async {
				let mut temp = self.create_temp().await?;
				for (path, dependency) in &output.dependencies {
					let dependency = self.evaluate(dependency).await?;
					let artifact = match dependency {
						Expression::Artifact(artifact) => artifact,
						_ => bail!(r#"Dependency must evaluate to an artifact."#),
					};
					self.add_dependency(&mut temp, path, artifact).await?;
				}
				Ok((name.clone(), temp))
			}))
			.await?
			.into_iter()
			.collect();

		// Evaluate the envs.
		let envs = self.evaluate(&process.env).await?;
		let envs = match envs {
			Expression::Map(envs) => envs,
			_ => bail!(r#"Argument "envs" must evaluate to a map."#),
		};
		let mut envs: BTreeMap<String, String> =
			try_join_all(envs.iter().map(|(key, value)| async move {
				Ok::<_, anyhow::Error>((key.as_ref().to_owned(), self.resolve(value).await?))
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
		let command = self.evaluate(&process.command).await?;
		let command = match command {
			Expression::Path(path) => path,
			_ => bail!("Command must be a path."),
		};
		let command_artifact = match *command.artifact {
			Expression::Artifact(artifact) => artifact,
			_ => bail!("Command artifact must be an artifact."),
		};
		let command_fragment = self.create_fragment(command_artifact).await?;
		let command_path = self.fragment_path(&command_fragment);
		let command = if let Some(path) = &command.path {
			command_path.join(path.as_ref())
		} else {
			command_path
		};

		// Evaluate the args.
		let args = self.evaluate(&process.args).await?;
		let args = match args {
			Expression::Array(array) => array,
			_ => bail!("Args must be an array."),
		};
		let args: Vec<String> = try_join_all(args.iter().map(|value| self.resolve(value))).await?;

		// Run the process.

		#[cfg(target_os = "linux")]
		self.run_linux_process(system, envs, command, args)
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

		// Create the output expression.
		let expression = if artifacts.len() == 1 {
			Expression::Artifact(artifacts.into_values().next().unwrap())
		} else {
			Expression::Map(
				artifacts
					.into_iter()
					.map(|(name, artifact)| (name.into(), Expression::Artifact(artifact)))
					.collect(),
			)
		};

		Ok(expression)
	}

	#[async_recursion]
	async fn resolve(self: &Arc<Self>, expression: &Expression) -> Result<String> {
		match expression {
			Expression::String(string) => Ok(string.as_ref().to_owned()),
			Expression::Template(template) => {
				let components =
					try_join_all(template.components.iter().map(|value| self.resolve(value)))
						.await?;
				let string = components.join("");
				Ok(string)
			},
			Expression::Path(path) => {
				let path_artifact = match *path.artifact {
					Expression::Artifact(artifact) => artifact,
					_ => bail!("Expected artifact."),
				};
				let fragment = self.create_fragment(path_artifact).await?;
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
