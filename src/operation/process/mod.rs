use crate::{
	artifact::{Artifact, ArtifactHash},
	operation::Process,
	system::System,
	value::{Template, TemplateComponent, Value},
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
	pub(super) async fn run_process(&self, process: &Process) -> Result<Value> {
		// Create the output temp path.
		let output_temp_path = self.create_temp_path();

		// Resolve the envs to strings with referenced paths.
		let env = if let Some(env) = &process.env {
			let output_temp_path = &output_temp_path;
			try_join_all(env.iter().map(|(key, value)| async move {
				let value = self
					.to_string_with_referenced_path_set(value, output_temp_path)
					.await?;
				Ok::<_, anyhow::Error>((key, value))
			}))
			.await?
		} else {
			vec![]
		};

		// Resolve the command to a string with referenced paths.
		let command = self
			.to_string_with_referenced_path_set(&process.command, &output_temp_path)
			.await?;

		// Resolve the args to strings with referenced paths.
		let args = if let Some(args) = &process.args {
			let output_temp_path = &output_temp_path;
			try_join_all(args.iter().map(|value| async move {
				let value = self
					.to_string_with_referenced_path_set(value, output_temp_path)
					.await?;
				Ok::<_, anyhow::Error>(value)
			}))
			.await?
		} else {
			vec![]
		};

		// Collect the referenced paths and get the strings for the envs, command, and args.
		let mut referenced_path_set = ReferencedPathSet::default();

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

		// Enable networking if the process is marked as unsafe.
		let network_enabled = process.is_unsafe;

		// Run the process.
		match process.system {
			System::Amd64Linux | System::Arm64Linux => {
				#[cfg(target_os = "linux")]
				{
					self.run_process_linux(
						process.system,
						env,
						command,
						args,
						referenced_path_set,
						network_enabled,
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
					self.run_process_macos(env, command, args, referenced_path_set, network_enabled)
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

		// Create the artifact value.
		let artifact = Value::Artifact(output_hash);

		Ok(artifact)
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
		template: &Template,
		output_temp_path: &Path,
	) -> Result<StringWithReferencedPathSet> {
		Ok(
			try_join_all(template.components.iter().map(|component| async move {
				match component {
					TemplateComponent::String(string) => Ok(StringWithReferencedPathSet {
						string: string.clone(),
						referenced_path_set: ReferencedPathSet::default(),
					}),

					TemplateComponent::Artifact(artifact_hash) => {
						let artifact_path = self.checkout_to_artifacts(*artifact_hash).await?;

						let string = artifact_path
							.to_str()
							.context("The path must be valid UTF-8.")?
							.to_owned();

						// Collect all dependency hashes and paths.
						let mut dependency_hashes = Vec::new();
						self.collect_dependency_hashes(*artifact_hash, &mut dependency_hashes)?;
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

						Ok::<_, anyhow::Error>(StringWithReferencedPathSet {
							string,
							referenced_path_set,
						})
					},

					TemplateComponent::Placeholder(placeholder) => {
						if placeholder.name == "output" {
							let mut referenced_path_set = ReferencedPathSet::default();
							let referenced_path = ReferencedPath {
								path: output_temp_path.to_owned(),
								mode: PathMode::ReadWriteCreate,
							};
							referenced_path_set.add(referenced_path);
							Ok::<_, anyhow::Error>(StringWithReferencedPathSet {
								string: output_temp_path.display().to_string(),
								referenced_path_set,
							})
						} else {
							bail!(r#"Invalid placeholder "{}"."#, placeholder.name);
						}
					},
				}
			}))
			.await?
			.into_iter()
			.collect(),
		)
	}

	/// Return all dependent artifacts recursively for an artifact.
	fn collect_dependency_hashes(
		&self,
		artifact_hash: ArtifactHash,
		dependency_hashes: &mut Vec<ArtifactHash>,
	) -> Result<()> {
		// Get the artifact.
		let artifact = self.get_artifact_local(artifact_hash)?;

		// Recurse into the children, if any.
		match artifact {
			Artifact::Directory(dir) => {
				for entry_hash in dir.entries.values() {
					self.collect_dependency_hashes(*entry_hash, dependency_hashes)?;
				}
			},

			Artifact::Dependency(dependency) => {
				// Add this dependency's artifact to the dependency hashes.
				dependency_hashes.push(dependency.artifact);

				// Recurse into the dependency.
				self.collect_dependency_hashes(dependency.artifact, dependency_hashes)?;
			},

			Artifact::File(_) | Artifact::Symlink(_) => {},
		};
		Ok(())
	}
}
