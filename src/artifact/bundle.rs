use super::{Artifact, Hash};
use crate::{
	directory::Directory,
	error::{Error, Result},
	file::File,
	path::{self, Path},
	return_error,
	symlink::Symlink,
	template, Instance,
};
use async_recursion::async_recursion;
use futures::future::try_join_all;
use once_cell::sync::Lazy;
use std::{collections::HashSet, sync::Arc};

static TANGRAM_ARTIFACTS_PATH: Lazy<Path> = Lazy::new(|| {
	Path::from_iter([
		path::Component::Normal(".tangram".to_string()),
		path::Component::Normal("artifacts".to_string()),
	])
});

impl Instance {
	pub async fn bundle(self: &Arc<Self>, artifact_hash: Hash) -> Result<Hash> {
		// Get the artifact.
		let artifact = self.get_artifact_local(artifact_hash)?;

		// Bundle the artifact.
		let bundled_artifact_hash = self.bundle_inner(artifact_hash, &Path::new()).await?;

		// Collect the references.
		let mut references = HashSet::default();
		artifact.collect_recursive_references_into(self, &mut references)?;

		// Bundle the references.
		let bundled_references = try_join_all(references.into_iter().map(|reference| {
			let tg = Arc::clone(self);
			async move {
				// Create the path for the reference at `TANGRAM_ARTIFACTS_PATH/HASH`.
				let path = TANGRAM_ARTIFACTS_PATH
					.clone()
					.join([path::Component::Normal(reference.to_string())]);

				// Bundle the reference.
				let bundled_reference = tg.bundle_inner(reference, &path).await?;

				Ok::<_, Error>(bundled_reference)
			}
		}))
		.await?;

		// Get the bundled artifact as a directory.
		let bundled_directory = self.get_artifact_local(bundled_artifact_hash)?;
		let Artifact::Directory(mut bundled_artifact) = bundled_directory else {
			return_error!("The artifact must be a directory.");
		};

		// Add the bundled references to the bundled artifact at `TANGRAM_ARTIFACTS_PATH`.
		let entries = bundled_references
			.into_iter()
			.map(|artifact_hash| (artifact_hash.to_string(), artifact_hash))
			.collect();
		let artifact = Artifact::Directory(Directory::new(entries));
		let artifact_hash = self.add_artifact(&artifact).await?;
		bundled_artifact
			.add(self, &TANGRAM_ARTIFACTS_PATH, artifact_hash)
			.await?;

		// Add the bundled artifact.
		let bundled_artifact_hash = self
			.add_artifact(&Artifact::Directory(bundled_artifact))
			.await?;

		Ok(bundled_artifact_hash)
	}

	/// Remove all references from an artifact recursively, rendering symlink targets to a relative path from `artifact_path` to `TANGRAM_ARTIFACTS_PATH/HASH`.
	#[async_recursion]
	async fn bundle_inner(
		self: &'async_recursion Arc<Self>,
		artifact_hash: Hash,
		artifact_path: &Path,
	) -> Result<Hash> {
		// Get the artifact.
		let artifact = self.get_artifact_local(artifact_hash)?;

		// Create the bundled artifact.
		let bundled_artifact = match artifact {
			// If the artifact is a directory, then recurse to bundle its entries.
			Artifact::Directory(directory) => {
				let entries = try_join_all(directory.entries().iter().map(|(name, hash)| {
					let tg = Arc::clone(self);
					async move {
						// Create the path for the entry.
						let path = artifact_path
							.clone()
							.join([path::Component::Normal(name.clone())]);

						// Bundle the entry.
						let bundled_entry_hash = tg.bundle_inner(*hash, &path).await?;

						Ok::<_, Error>((name.clone(), bundled_entry_hash))
					}
				}))
				.await?
				.into_iter()
				.collect();

				Artifact::Directory(Directory::new(entries))
			},

			// If the artifact is a file, then clear its references.
			Artifact::File(file) => {
				Artifact::File(File::new(file.blob_hash(), file.executable(), vec![]))
			},

			// If the artifact is a file, then render its target to refer to the artifacts path.
			Artifact::Symlink(symlink) => {
				// Render the target.
				let target = symlink
					.target
					.render(|component| async move {
						match component {
							template::Component::String(string) => Ok(string.into()),

							template::Component::Artifact(artifact_hash) => {
								// Render an artifact component with the diff from the path's parent to the referenced artifact's bundled path.
								let bundle_path = TANGRAM_ARTIFACTS_PATH
									.clone()
									.join([path::Component::Normal(artifact_hash.to_string())]);
								let path = bundle_path
									.diff(&artifact_path.clone().join([path::Component::ParentDir]))
									.to_string()
									.into();
								Ok(path)
							},

							template::Component::Placeholder(_) => {
								return_error!(
									"Cannot bundle a symlink whose target hash placeholders."
								);
							},
						}
					})
					.await?
					.into();

				Artifact::Symlink(Symlink::new(target))
			},
		};

		// Add the bundled artifact.
		let bundled_artifact_hash = self.add_artifact(&bundled_artifact).await?;

		Ok(bundled_artifact_hash)
	}
}
