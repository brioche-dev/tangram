use crate::{
	builder::State,
	command::{Command, PathMode},
	expression::{Expression, Process},
	hash::Hash,
};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use futures::future::{try_join3, try_join_all};
use std::{
	collections::{BTreeMap, HashMap},
	path::PathBuf,
};

impl State {
	pub(super) async fn evaluate_process(&self, hash: Hash, process: &Process) -> Result<Hash> {
		// Evaluate the envs, command, and args.
		let (envs, command, args) = try_join3(
			self.evaluate(process.env, hash),
			self.evaluate(process.command, hash),
			self.evaluate(process.args, hash),
		)
		.await?;

		// Convert the envs to strings.
		let envs = match self.get_expression_local(envs)? {
			Expression::Map(envs) => envs,
			_ => bail!(r#"Argument "envs" must evaluate to a map."#),
		};
		let mut envs: BTreeMap<String, StringWithPaths> =
			try_join_all(envs.iter().map(|(key, hash)| async move {
				let key = key.as_ref().to_owned();
				let value = self.to_string_with_paths(*hash).await?;
				Ok::<_, anyhow::Error>((key, value))
			}))
			.await?
			.into_iter()
			.collect();

		// Create a temp for the output and add it to the envs.
		let out_temp_path = self.create_temp_path();
		envs.insert(
			"out".to_owned(),
			StringWithPaths {
				string: out_temp_path.display().to_string(),
				paths: [(out_temp_path.clone(), PathMode::ReadWriteCreate)].into(),
			},
		);

		// Convert the args to strings.
		let args = match self.get_expression_local(args)? {
			Expression::Array(array) => array,
			_ => bail!("Args must evaluate to an array."),
		};
		let args = try_join_all(args.iter().copied().map(|hash| async move {
			let string = self.to_string_with_paths(hash).await?;
			Ok::<_, anyhow::Error>(string)
		}))
		.await
		.context("Failed to resolve the args.")?;

		// Conver the command to a string.
		let command = self.to_string_with_paths(command).await?;

		// If networking is required, ensure that there is an output hash or that the process is marked as unsafe.
		let is_network_access_allowed = process.hash.is_some() || process.is_unsafe;
		if process.network && !is_network_access_allowed {
			bail!("Process is not allowed to access the network! If the process requires network access, the `hash` field must be set or the process must be marked as unsafe.");
		}

		// Create a new working dir.
		let current_dir = self.create_temp_path();
		tokio::fs::create_dir_all(&current_dir).await?;

		// Build an iterator of all paths referred to by the command.
		let current_dir_paths_iter = [(&current_dir, &PathMode::ReadWrite)].into_iter();
		let envs_paths_iter = envs.values().flat_map(|value| value.paths.iter());
		let command_paths_iter = command.paths.iter();
		let args_paths_iter = args.iter().flat_map(|arg| arg.paths.iter());
		let paths_iter = current_dir_paths_iter
			.chain(envs_paths_iter)
			.chain(command_paths_iter)
			.chain(args_paths_iter);

		// Collect the paths, taking the most permissive of any duplicate permissions.
		let mut paths = HashMap::new();
		for (path, &mode) in paths_iter {
			paths
				.entry(path.clone())
				.and_modify(|current_mode| *current_mode = mode.max(*current_mode))
				.or_insert(mode);
		}

		// Get the strings for the envs, command, and args.
		let envs = envs
			.into_iter()
			.map(|(key, value)| (key, value.string))
			.collect();
		let command = PathBuf::from(command.string);
		let args = args.into_iter().map(|arg| arg.string).collect();

		// Create the command.
		let command = Command {
			#[cfg(target_os = "linux")]
			chroot_path: self.create_temp_path(),
			current_dir,
			envs,
			command,
			args,
			paths,
			enable_network_access: process.network,
		};

		// Run the command.
		command.run().await.context("Failed to run the process.")?;

		// Create the output.
		let output_hash = self.checkin(&out_temp_path).await?;

		// If a hash was provided in the expression, verify the output hash matches it.
		if let Some(expected_hash) = process.hash {
			if expected_hash != output_hash {
				bail!("Hash mismatch in process!\nExpected: {expected_hash}\nReceived: {output_hash}\n");
			}
		}

		Ok(output_hash)
	}
}

// A `StringWithPaths` contains a string and a set of paths that it refers to.
struct StringWithPaths {
	string: String,
	paths: HashMap<PathBuf, PathMode>,
}

impl State {
	#[async_recursion]
	async fn to_string_with_paths(&self, hash: Hash) -> Result<StringWithPaths> {
		let expression = self.get_expression_local(hash)?;
		match expression {
			Expression::String(string) => Ok(StringWithPaths {
				string: string.as_ref().to_owned(),
				paths: HashMap::new(),
			}),

			Expression::Artifact(_) => {
				// Checkout the artifact.
				let artifact_path = self.checkout_to_artifacts(hash).await?;
				let string = artifact_path
					.to_str()
					.context("The path must be valid UTF-8.")?
					.to_owned();

				// Collect all transitive artifact hashes.
				let mut artifact_hashes = Vec::new();
				self.collect_into_artifact_hashes(hash, &mut artifact_hashes)?;

				// Checkout all artifacts.
				let artifact_paths = try_join_all(
					artifact_hashes
						.into_iter()
						.map(|hash| self.checkout_to_artifacts(hash)),
				)
				.await?;

				// Include the artifact and all dependencies as read-only.
				let paths = artifact_paths
					.into_iter()
					.map(|artifact_path| (artifact_path, PathMode::Read))
					.collect();

				Ok(StringWithPaths { string, paths })
			},

			Expression::Template(template) => {
				let components = try_join_all(template.components.iter().copied().map(
					|component_hash| async move {
						let component_hash = self.evaluate(component_hash, hash).await?;
						let string_with_paths = self.to_string_with_paths(component_hash).await?;
						Ok::<_, anyhow::Error>(string_with_paths)
					},
				))
				.await?;
				let string_with_paths = components.into_iter().fold(
					StringWithPaths {
						string: "".to_owned(),
						paths: HashMap::new(),
					},
					|mut a, b| {
						a.string.push_str(&b.string);
						a.paths.extend(b.paths);
						a
					},
				);
				Ok(string_with_paths)
			},

			_ => bail!("The expression must be a string, artifact, or template."),
		}
	}

	// Return all dependent artifacts recursively for an artifact.
	fn collect_into_artifact_hashes(
		&self,
		hash: Hash,
		artifact_hashes: &mut Vec<Hash>,
	) -> Result<()> {
		let expression = self.get_expression_local(hash)?;
		match expression {
			Expression::Artifact(artifact) => {
				// Add the artifact itself as a dependency.
				artifact_hashes.push(hash);

				// Get all dependent artifacts from the root.
				self.collect_into_artifact_hashes(artifact.root, artifact_hashes)?;
			},

			Expression::Directory(dir) => {
				// Get the dependencies of each entry.
				for entry_hash in dir.entries.values() {
					self.collect_into_artifact_hashes(*entry_hash, artifact_hashes)?;
				}
			},

			// Files and symlinks aren't dependencies.
			Expression::File(_) | Expression::Symlink(_) => {},

			// Recurse into dependencies.
			Expression::Dependency(dep) => {
				self.collect_into_artifact_hashes(dep.artifact, artifact_hashes)?;
			},

			_ => {
				bail!(
					r#"Tried to get dependent artifacts for a non-filesystem expression "{hash}"."#
				);
			},
		};
		Ok(())
	}
}
