use crate::{
	artifact::Artifact,
	expression,
	object::Dependency,
	server::runtime,
	server::{temp::Temp, Server},
	value::Value,
};
use anyhow::{bail, Result};
use futures::future::try_join_all;
use std::{collections::BTreeMap, sync::Arc};

impl Server {
	pub async fn evaluate_process(self: &Arc<Self>, process: expression::Process) -> Result<Value> {
		match process {
			expression::Process::Arm64Macos(process) => self.evaluate_unix_process(process).await,
			expression::Process::Js(process) => self.evaluate_js_process(process).await,
			_ => unimplemented!(),
		}
	}

	pub async fn evaluate_unix_process(
		self: &Arc<Self>,
		process: expression::UnixProcess,
	) -> Result<Value> {
		let crate::expression::UnixProcess { command, args, .. } = process;

		// Evaluate the envs.
		let envs = self.evaluate(*process.env).await?;
		let envs = match envs {
			Value::Map(envs) => envs,
			_ => bail!(r#"Argument "envs" must evaluate to a map."#),
		};
		let mut envs: BTreeMap<String, String> = envs
			.into_iter()
			.map(|(key, value)| {
				let value = match value {
					Value::String(value) => value,
					_ => bail!(r#"Value in "envs" must evaluate to a string."#),
				};
				Ok((key, value))
			})
			.collect::<Result<_>>()?;

		// Create the temps for the outputs and add their dependencies.
		let temps: BTreeMap<String, Temp> =
			try_join_all(process.outputs.into_iter().map(|(name, output)| {
				async {
					let temp = self.create_temp().await?;
					for (path, dependency) in output.dependencies {
						let dependency = self.evaluate(*dependency).await?;
						let dependency = match dependency {
							Value::Artifact(artifact) => Dependency { artifact },
							_ => bail!(r#"Dependency must evaluate to an artifact."#),
						};
						self.add_dependency(&temp, &path, dependency).await?;
					}
					Ok((name, temp))
				}
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
		let command_fragment = self.create_fragment(&command.artifact).await?;
		let command_path = command_fragment.path();
		let command_path = if let Some(path) = &command.path {
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
		let args: Vec<String> = args
			.into_iter()
			.map(|arg| {
				match arg {
					Value::String(string) => Ok(string),
					_ => bail!("Arg must be a string"),
				}
			})
			.collect::<Result<_>>()?;

		// Run the process.
		let mut process = tokio::process::Command::new(command_path)
			.envs(envs)
			.args(args)
			.spawn()?;
		process.wait().await?;

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

	pub async fn evaluate_js_process(
		self: &Arc<Self>,
		process: expression::JsProcess,
	) -> Result<Value> {
		// Create a JS runtime.
		let runtime = runtime::js::Runtime::new(self);

		// Run the process.
		let expression = runtime.run(process).await??;

		// Evaluate the resulting expression.
		let value = self.evaluate(expression).await?;

		Ok(value)
	}
}
