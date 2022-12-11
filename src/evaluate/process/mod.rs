use crate::{
	expression::{Expression, Process},
	hash::Hash,
	system::System,
	State,
};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use futures::{future::try_join_all, FutureExt};
use std::{
	collections::BTreeMap,
	path::{Path, PathBuf},
};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

impl State {
	#[allow(clippy::too_many_lines)]
	pub(super) async fn evaluate_process(&self, hash: Hash, process: &Process) -> Result<Hash> {
		// Create the output temp path.
		let output_temp_path = self.create_temp_path();

		// Create a temp path for the working directory and check out the working directory artifact if provided.
		let working_directory_path = self.create_temp_path();
		tokio::fs::create_dir_all(&working_directory_path).await?;
		let working_directory_hash = self.evaluate(process.working_directory, hash).await?;
		let working_directory = self.get_expression_local(working_directory_hash)?;
		match working_directory {
			Expression::Null(_) => {},
			Expression::Directory(_) => {
				self.checkout(working_directory_hash, &working_directory_path, None)
					.await?;
			},
			_ => {
				bail!("The working directory must be a directory or null.");
			},
		};
		let mut referenced_path_set = ReferencedPathSet::default();
		referenced_path_set.add(ReferencedPath {
			path: working_directory_path.clone(),
			mode: PathMode::ReadWrite,
		});
		let working_directory = StringWithReferencedPathSet::new(
			working_directory_path.display().to_string(),
			referenced_path_set,
		);

		// Evaluate the envs to strings with referenced paths.
		let env_hash = self.evaluate(process.env, hash).await?;
		let env = self.get_expression_local(env_hash)?;
		let env = match env {
			Expression::Null(_) => {
				vec![]
			},
			Expression::Map(env) => {
				let output_temp_path = &output_temp_path;
				try_join_all(env.into_iter().map(|(key, value)| async move {
					let value = self
						.to_string_with_referenced_path_set(value, output_temp_path)
						.await?;
					Ok::<_, anyhow::Error>((key, value))
				}))
				.await?
			},
			_ => bail!(r#"Argument "envs" must evaluate to a map."#),
		};

		// Evaluate the command to a string with referenced paths.
		let command = self.evaluate(process.command, hash).await?;
		let command = self
			.to_string_with_referenced_path_set(command, &output_temp_path)
			.await?;

		// Evaluate the args to strings with referenced paths.
		let args = self.evaluate(process.args, hash).await?;
		let args = self.get_expression_local(args)?;
		let args = match args {
			Expression::Null(_) => vec![],
			Expression::Array(args) => {
				let output_temp_path = &output_temp_path;
				try_join_all(args.into_iter().map(|arg| async move {
					let arg = self
						.to_string_with_referenced_path_set(arg, output_temp_path)
						.await?;
					Ok::<_, anyhow::Error>(arg)
				}))
				.await?
			},
			_ => bail!("Args must be an array."),
		};

		// Collect the referenced paths and get the strings for the envs, command, working directory, and args.
		let mut referenced_path_set = ReferencedPathSet::default();

		referenced_path_set.extend(working_directory.referenced_path_set);
		let working_directory = working_directory.string;

		let env = env
			.into_iter()
			.map(|(key, value)| {
				referenced_path_set.extend(value.referenced_path_set);
				(key.to_string(), value.string)
			})
			.collect();

		referenced_path_set.extend(command.referenced_path_set);
		let command = command.string;

		let args = args
			.into_iter()
			.map(|value| {
				referenced_path_set.extend(value.referenced_path_set);
				value.string
			})
			.collect();

		// If networking is required, ensure that there is an output hash or that the process is marked as unsafe.
		let is_network_access_allowed = process.checksum.is_some() || process.is_unsafe;
		if process.network && !is_network_access_allowed {
			bail!("The process has network access enabled, but has not provided a checksum or set the unsafe flag.");
		}

		// Run the process.
		match process.system {
			System::Amd64Linux | System::Arm64Linux => {
				#[cfg(target_os = "linux")]
				{
					self.run_process_linux(
						working_directory,
						env,
						command,
						args,
						referenced_path_set,
						process.network,
					)
					.boxed()
				}
				#[cfg(not(target_os = "linux"))]
				{
					anyhow::bail!("A Linux process cannot run on a non-Linux host.");
				}
			},
			System::Amd64Macos | System::Arm64Macos => {
				#[cfg(target_os = "macos")]
				{
					self.run_process_macos(
						working_directory,
						env,
						command,
						args,
						referenced_path_set,
						process.network,
					)
					.boxed()
				}
				#[cfg(not(target_os = "macos"))]
				{
					anyhow::bail!("A macOS process cannot run on a non-macOS host.");
				}
			},
		}
		.await?;

		// Check in the output temp path.
		let output_hash = self
			.checkin(&output_temp_path)
			.await
			.context("Failed to check in the output.")?;

		Ok(output_hash)
	}
}

#[derive(Clone, Debug, Default)]
pub struct StringWithReferencedPathSet {
	pub string: String,
	pub referenced_path_set: ReferencedPathSet,
}

#[derive(Clone, Debug, Default)]
pub struct ReferencedPathSet(BTreeMap<PathBuf, ReferencedPath>);

#[derive(Debug, Clone)]
pub struct ReferencedPath {
	pub path: PathBuf,
	pub mode: PathMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PathMode {
	Read = 0,
	ReadWrite = 1,
	ReadWriteCreate = 2,
}

impl StringWithReferencedPathSet {
	#[must_use]
	pub fn new(string: String, referenced_paths: ReferencedPathSet) -> StringWithReferencedPathSet {
		StringWithReferencedPathSet {
			string,
			referenced_path_set: referenced_paths,
		}
	}
}

impl FromIterator<StringWithReferencedPathSet> for StringWithReferencedPathSet {
	fn from_iter<T: IntoIterator<Item = StringWithReferencedPathSet>>(iter: T) -> Self {
		let mut string_with_referenced_path_set = StringWithReferencedPathSet::default();
		string_with_referenced_path_set.extend(iter);
		string_with_referenced_path_set
	}
}

impl Extend<StringWithReferencedPathSet> for StringWithReferencedPathSet {
	fn extend<T: IntoIterator<Item = StringWithReferencedPathSet>>(&mut self, iter: T) {
		for item in iter {
			self.string.push_str(&item.string);
			self.referenced_path_set.extend(item.referenced_path_set);
		}
	}
}

impl ReferencedPathSet {
	pub fn add(&mut self, entry: ReferencedPath) {
		self.0
			.entry(entry.path.clone())
			.and_modify(|current_entry| current_entry.mode = entry.mode.max(current_entry.mode))
			.or_insert(entry);
	}
}

impl FromIterator<ReferencedPath> for ReferencedPathSet {
	fn from_iter<T: IntoIterator<Item = ReferencedPath>>(iter: T) -> Self {
		let mut referenced_paths = ReferencedPathSet::default();
		referenced_paths.extend(iter);
		referenced_paths
	}
}

impl Extend<ReferencedPath> for ReferencedPathSet {
	fn extend<T: IntoIterator<Item = ReferencedPath>>(&mut self, iter: T) {
		for entry in iter {
			self.add(entry);
		}
	}
}

impl<'a> IntoIterator for &'a ReferencedPathSet {
	type Item = &'a ReferencedPath;

	type IntoIter = std::collections::btree_map::Values<'a, PathBuf, ReferencedPath>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.values()
	}
}

impl<'a> IntoIterator for &'a mut ReferencedPathSet {
	type Item = &'a mut ReferencedPath;

	type IntoIter = std::collections::btree_map::ValuesMut<'a, PathBuf, ReferencedPath>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.values_mut()
	}
}

impl IntoIterator for ReferencedPathSet {
	type Item = ReferencedPath;

	type IntoIter = std::collections::btree_map::IntoValues<PathBuf, ReferencedPath>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.into_values()
	}
}

impl State {
	#[async_recursion]
	pub async fn to_string_with_referenced_path_set(
		&self,
		hash: Hash,
		output_temp_path: &Path,
	) -> Result<StringWithReferencedPathSet> {
		let expression = self.get_expression_local(hash)?;
		match expression {
			Expression::String(string) => Ok(StringWithReferencedPathSet {
				string: string.as_ref().to_owned(),
				referenced_path_set: ReferencedPathSet::default(),
			}),

			Expression::Directory(_)
			| Expression::File(_)
			| Expression::Symlink(_)
			| Expression::Dependency(_) => {
				// Checkout the artifact.
				let artifact_path = self.checkout_to_artifacts(hash).await?;
				let string = artifact_path
					.to_str()
					.context("The path must be valid UTF-8.")?
					.to_owned();

				// Collect all dependency hashes and paths.
				let mut dependency_hashes = Vec::new();
				self.collect_dependency_hashes(hash, &mut dependency_hashes)?;
				let dependency_paths = try_join_all(
					dependency_hashes
						.into_iter()
						.map(|hash| self.checkout_to_artifacts(hash)),
				)
				.await?;

				// Include the artifact path and all its dependency paths as read-only.
				let referenced_path_set = [artifact_path]
					.into_iter()
					.chain(dependency_paths.into_iter())
					.map(|path| ReferencedPath {
						path,
						mode: PathMode::Read,
					})
					.collect();

				Ok(StringWithReferencedPathSet {
					string,
					referenced_path_set,
				})
			},

			Expression::Template(template) => {
				Ok(try_join_all(template.components.iter().copied().map(
					|component_hash| async move {
						let component_hash = self.evaluate(component_hash, hash).await?;
						let string_with_referenced_path_set = self
							.to_string_with_referenced_path_set(component_hash, output_temp_path)
							.await?;
						Ok::<_, anyhow::Error>(string_with_referenced_path_set)
					},
				))
				.await?
				.into_iter()
				.collect())
			},

			Expression::Placeholder(placeholder) if placeholder.name == "output" => {
				let mut referenced_path_set = ReferencedPathSet::default();
				referenced_path_set.add(ReferencedPath {
					path: output_temp_path.to_owned(),
					mode: PathMode::ReadWriteCreate,
				});
				Ok(StringWithReferencedPathSet {
					string: output_temp_path.display().to_string(),
					referenced_path_set,
				})
			},

			_ => {
				bail!(
					r#"The expression must be a string, a placeholder named "output", an artifact, or a template."#
				);
			},
		}
	}

	/// Return all dependent artifacts recursively for an artifact.
	fn collect_dependency_hashes(
		&self,
		hash: Hash,
		dependency_hashes: &mut Vec<Hash>,
	) -> Result<()> {
		// Get the expression.
		let expression = self.get_expression_local(hash)?;

		// Recurse into the children, if any.
		match expression {
			Expression::Directory(dir) => {
				for entry_hash in dir.entries.values() {
					self.collect_dependency_hashes(*entry_hash, dependency_hashes)?;
				}
			},

			Expression::Dependency(dependency) => {
				// Add this dependency's artifact to the dependency hashes.
				dependency_hashes.push(dependency.artifact);

				// Recurse into the dependency.
				self.collect_dependency_hashes(dependency.artifact, dependency_hashes)?;
			},

			Expression::File(_) | Expression::Symlink(_) => {},

			_ => {
				bail!(
					r#"Tried to get dependent artifacts for a non-filesystem expression "{hash}"."#
				);
			},
		};
		Ok(())
	}
}
