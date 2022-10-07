use crate::{builder, evaluators::Evaluator, expression::Expression, hash::Hash, system::System};
use anyhow::{anyhow, bail, Context, Result};
use async_recursion::async_recursion;
use async_trait::async_trait;
use futures::future::{try_join3, try_join_all};
use std::{
	collections::{BTreeMap, HashMap},
	path::PathBuf,
};

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
		let mut envs: BTreeMap<String, StringWithPaths> =
			try_join_all(envs.iter().map(|(key, hash)| async move {
				let key = key.as_ref().to_owned();
				let value = self.to_string_with_paths(builder, *hash).await?;
				Ok::<_, anyhow::Error>((key, value))
			}))
			.await?
			.into_iter()
			.collect();

		// Create a temp for the output and include the temps path in the sandbox.
		let out_temp_path = builder.create_temp_path();
		envs.insert(
			"out".to_owned(),
			StringWithPaths {
				string: out_temp_path.display().to_string(),
				paths: [(out_temp_path.clone(), SandboxPathMode::ReadWriteCreate)]
					.into_iter()
					.collect(),
			},
		);

		// Resolve the args.
		let args = match builder.get_expression(args).await? {
			Expression::Array(array) => array,
			_ => bail!("Args must evaluate to an array."),
		};
		let args: Vec<StringWithPaths> =
			try_join_all(args.iter().copied().map(|hash| async move {
				let string = self.to_string_with_paths(builder, hash).await?;
				Ok::<_, anyhow::Error>(string)
			}))
			.await
			.context("Failed to resolve the args.")?;

		// Resolve the command.
		let command = self.to_string_with_paths(builder, command).await?;

		// Only allow network access if an ouput hash was provided.
		let expected_hash = process.hash;
		let enable_network_access = expected_hash.is_some();

		// Create a new working dir.
		let working_dir = builder.create_temp_path();
		tokio::fs::create_dir_all(&working_dir).await?;

		// Create the command.
		let command = SandboxedCommand {
			builder: builder.clone(),
			parent_hash: hash,
			system: process.system,
			command,
			args,
			envs,
			working_dir,
			enable_network_access,
		};

		// Run the command.
		command.run().await?;

		// Create the output.
		let output_hash = builder.checkin(&out_temp_path).await?;

		// Verify output hash matches if provided in the expression
		match expected_hash {
			Some(expected_hash) if expected_hash != output_hash => {
				bail!(
					"Hash mismatch in process!\nExpected: {}\nReceived: {}\n",
					expected_hash,
					output_hash,
				)
			},
			_ => {},
		}

		Ok(Some(output_hash))
	}
}

// A string value used in a process with a list of artifact or temp paths that need to be included in the sandbox when this value is used.
struct StringWithPaths {
	string: String,
	paths: HashMap<PathBuf, SandboxPathMode>,
}

impl StringWithPaths {
	fn empty() -> Self {
		Self {
			string: String::new(),
			paths: HashMap::new(),
		}
	}

	fn concat(&mut self, other: Self) {
		self.string.push_str(&other.string);
		self.paths.extend(other.paths);
	}
}

// The fully resolved command for a process with arguments and environment variables, plus all paths required to run the process in the sandbox.
struct SandboxedCommand {
	pub builder: builder::Shared,
	pub parent_hash: Hash,
	pub system: System,
	pub command: StringWithPaths,
	pub args: Vec<StringWithPaths>,
	pub envs: BTreeMap<String, StringWithPaths>,
	pub working_dir: PathBuf,
	pub enable_network_access: bool,
}

// The mode used to mount a path in the sandbox. Ordered from least permissive to most permissive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SandboxPathMode {
	Read,
	ReadWrite,
	ReadWriteCreate,
}

impl SandboxedCommand {
	fn paths(&self) -> HashMap<PathBuf, SandboxPathMode> {
		// Build an iterator of all paths used in the command.
		let all_paths = [(&self.working_dir, &SandboxPathMode::ReadWrite)]
			.into_iter()
			.chain(self.command.paths.iter())
			.chain(self.args.iter().flat_map(|arg| arg.paths.iter()))
			.chain(self.envs.values().flat_map(|value| value.paths.iter()));

		let mut paths = HashMap::new();

		// Take the most permissive mode for duplicate paths.
		for (path, &mode) in all_paths {
			paths
				.entry(path.clone())
				.and_modify(|current_mode| *current_mode = mode.max(*current_mode))
				.or_insert(mode);
		}

		paths
	}
}

impl Process {
	#[async_recursion]
	async fn to_string_with_paths(
		&self,
		builder: &builder::Shared,
		hash: Hash,
	) -> Result<StringWithPaths> {
		let expression = builder.get_expression(hash).await?;
		match expression {
			Expression::String(string) => Ok(StringWithPaths {
				string: string.to_string(),
				paths: HashMap::new(),
			}),
			Expression::Artifact(_) => {
				let artifact_path = builder.checkout_to_artifacts(hash).await?;
				let artifact_path_string = artifact_path
					.to_str()
					.ok_or_else(|| anyhow!("The path must be valid UTF-8."))?
					.to_owned();
				Ok(StringWithPaths {
					string: artifact_path_string,
					paths: [(artifact_path, SandboxPathMode::Read)]
						.into_iter()
						.collect(),
				})
			},
			Expression::Template(template) => {
				let component_results = try_join_all(template.components.iter().copied().map(
					|component_hash| async move {
						let component_hash = builder.evaluate(component_hash, hash).await?;
						let resolved = self.to_string_with_paths(builder, component_hash).await?;
						Ok::<_, anyhow::Error>(resolved)
					},
				))
				.await?;

				let result =
					component_results
						.into_iter()
						.fold(StringWithPaths::empty(), |mut a, b| {
							a.concat(b);
							a
						});
				Ok(result)
			},
			_ => bail!("The expression to resolve must be a string, artifact, or template."),
		}
	}
}
