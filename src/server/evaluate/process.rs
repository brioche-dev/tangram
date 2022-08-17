use crate::{expression, server::runtime, server::Server, value::Value};
use anyhow::{bail, Result};
use std::sync::Arc;

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

		// Create the temp for the output.
		let temp = self.create_temp().await?;
		let temp_path = self.temp_path(&temp);
		let envs = [("OUTPUT", temp_path)];

		// Run the process.
		let mut process = tokio::process::Command::new(command_path)
			.envs(envs)
			.args(args)
			.spawn()?;
		process.wait().await?;

		// Checkin the temp.
		let artifact = self.checkin_temp(temp).await?;

		// Create the output value.
		let value = Value::Artifact(artifact);

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
