use super::{Component, Template};
use crate::{
	artifact::{self, Artifact},
	error::{bail, Context, Result},
	os, Instance,
};
use futures::future::try_join_all;
use std::{
	collections::{BTreeMap, HashSet},
	sync::Arc,
};

#[derive(Clone, Debug, Default)]
pub struct Output {
	pub string: String,
	pub paths: HashSet<Path, fnv::FnvBuildHasher>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Path {
	pub host_path: os::PathBuf,
	pub guest_path: os::PathBuf,
	pub read: bool,
	pub write: bool,
	pub create: bool,
}

impl FromIterator<Output> for Output {
	fn from_iter<T: IntoIterator<Item = Output>>(iter: T) -> Self {
		let mut output = Output::default();
		output.extend(iter);
		output
	}
}

impl Extend<Output> for Output {
	fn extend<T: IntoIterator<Item = Output>>(&mut self, iter: T) {
		for item in iter {
			self.string.push_str(&item.string);
			self.paths.extend(item.paths);
		}
	}
}

impl Instance {
	pub async fn render(
		self: &Arc<Self>,
		template: &Template,
		placeholder_values: &BTreeMap<String, Path>,
	) -> Result<Output> {
		Ok(
			try_join_all(template.components.iter().map(|component| async move {
				match component {
					Component::String(string) => Ok(Output {
						string: string.clone(),
						paths: HashSet::default(),
					}),

					Component::Artifact(artifact_hash) => {
						// Check out the artifact.
						let artifact_host_path = self.check_out_internal(*artifact_hash).await?;

						// Get the host path as a string.
						let string = artifact_host_path
							.to_str()
							.context("The path must be valid UTF-8.")?
							.to_owned();

						// Collect all referenced artifact hashes and paths.
						let mut referenced_artifact_hashes = Vec::new();
						self.collect_referenced_artifact_hashes(
							*artifact_hash,
							&mut referenced_artifact_hashes,
						)?;
						let referenced_artifact_host_paths = try_join_all(
							referenced_artifact_hashes
								.into_iter()
								.map(|hash| self.check_out_internal(hash)),
						)
						.await?;

						// Create a set of paths including the artifact and all its referenced artifacts.
						let paths = [artifact_host_path]
							.into_iter()
							.chain(referenced_artifact_host_paths.into_iter())
							.map(|host_path| Path {
								host_path: host_path.clone(),
								guest_path: host_path,
								read: true,
								write: false,
								create: false,
							})
							.collect();

						Ok::<_, anyhow::Error>(Output { string, paths })
					},

					Component::Placeholder(placeholder) => {
						let Some(placeholder_value) = placeholder_values.get(&placeholder.name).cloned() else {
							bail!(r#"Invalid placeholder "{}"."#, placeholder.name);
						};
						let string = placeholder_value.guest_path.display().to_string();
						let mut paths = HashSet::default();
						paths.insert(placeholder_value);
						Ok::<_, anyhow::Error>(Output { string, paths })
					},
				}
			}))
			.await?
			.into_iter()
			.collect(),
		)
	}
}

impl Instance {
	/// Return all artifacts an artifact references recursively.
	fn collect_referenced_artifact_hashes(
		&self,
		artifact_hash: artifact::Hash,
		referenced_artifact_hashes: &mut Vec<artifact::Hash>,
	) -> Result<()> {
		// Get the artifact.
		let artifact = self.get_artifact_local(artifact_hash)?;

		// Recurse into the children.
		match artifact {
			Artifact::Directory(dir) => {
				for entry_hash in dir.entries.values() {
					self.collect_referenced_artifact_hashes(
						*entry_hash,
						referenced_artifact_hashes,
					)?;
				}
			},

			Artifact::File(_) | Artifact::Symlink(_) => {},

			Artifact::Reference(dependency) => {
				// Add this reference's artifact to the referenced artifact hashes.
				referenced_artifact_hashes.push(dependency.artifact_hash);

				// Recurse into the referenced artifact.
				self.collect_referenced_artifact_hashes(
					dependency.artifact_hash,
					referenced_artifact_hashes,
				)?;
			},
		};

		Ok(())
	}
}
