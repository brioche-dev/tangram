use crate::{builder, evaluators::Evaluator, expression::Expression, hash::Hash};
use anyhow::{anyhow, bail, Context, Result};
use async_recursion::async_recursion;
use async_trait::async_trait;
use futures::future::{try_join3, try_join_all};
use std::collections::BTreeMap;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

pub struct Process {}

impl Process {
	#[must_use]
	pub fn new() -> Process {
		Process {}
	}
}

impl Default for Process {
	fn default() -> Self {
		Process::new()
	}
}

#[async_trait]
impl Evaluator for Process {
	async fn evaluate(
		&self,
		builder: &builder::Shared,
		hash: Hash,
		expression: &Expression,
	) -> Result<Option<Hash>> {
		let process = if let Expression::Process(process) = expression {
			process
		} else {
			return Ok(None);
		};

		// Evaluate the envs, command, and args.
		let (envs, command, args) = try_join3(
			builder.evaluate(process.env, hash),
			builder.evaluate(process.command, hash),
			builder.evaluate(process.args, hash),
		)
		.await?;

		// Resolve the envs.
		let envs = match builder.get_expression(envs).await? {
			Expression::Map(envs) => envs,
			_ => bail!(r#"Argument "envs" must evaluate to a map."#),
		};
		let mut envs: BTreeMap<String, String> =
			try_join_all(envs.iter().map(|(key, hash)| async move {
				let key = key.as_ref().to_owned();
				let string = self.resolve(builder, *hash).await?;
				Ok::<_, anyhow::Error>((key, string))
			}))
			.await?
			.into_iter()
			.collect();

		// Create a temp for the output.
		let out_temp_path = builder.create_temp_path();
		envs.insert("out".to_owned(), out_temp_path.display().to_string());

		// Resolve the command.
		let command = self.resolve(builder, command).await?;

		// Resolve the args.
		let args = match builder.get_expression(args).await? {
			Expression::Array(array) => array,
			_ => bail!("Args must evaluate to an array."),
		};
		let args: Vec<String> = try_join_all(args.iter().copied().map(|hash| async move {
			let string = self.resolve(builder, hash).await?;
			Ok::<_, anyhow::Error>(string)
		}))
		.await
		.context("Failed to resolve the args.")?;

		// Run the process.

		#[cfg(target_os = "linux")]
		self.run_linux_process(builder, process.system, envs, command.into(), args, hash)
			.await
			.context("Failed to run the process.")?;

		#[cfg(target_os = "macos")]
		self.run_macos_process(builder, process.system, envs, command.into(), args)
			.await
			.context("Failed to run the process.")?;

		// Create the output.
		let output_hash = builder.checkin(&out_temp_path).await?;

		Ok(Some(output_hash))
	}
}

impl Process {
	#[async_recursion]
	async fn resolve(&self, builder: &builder::Shared, hash: Hash) -> Result<String> {
		let expression = builder.get_expression(hash).await?;
		match expression {
			Expression::String(string) => Ok(string.as_ref().to_owned()),
			Expression::Artifact(_) => {
				let artifact_path = builder.checkout_to_artifacts(hash).await?;
				let artifact_path_string = artifact_path
					.to_str()
					.ok_or_else(|| anyhow!("The path must be valid UTF-8."))?
					.to_owned();
				Ok(artifact_path_string)
			},
			Expression::Template(template) => {
				let components = try_join_all(template.components.iter().copied().map(
					|component_hash| async move {
						let component_hash = builder.evaluate(component_hash, hash).await?;
						let string = self.resolve(builder, component_hash).await?;
						Ok::<_, anyhow::Error>(string)
					},
				))
				.await?;
				let string = components.join("");
				Ok(string)
			},
			_ => bail!("The expression to resolve must be a string, artifact, or template."),
		}
	}
}
