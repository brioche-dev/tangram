use std::collections::{btree_map, BTreeMap};

use async_recursion::async_recursion;
use once_cell::sync::Lazy;

use super::{Artifact, Hash};
use crate::{
	directory::Directory,
	error::Result,
	file::File,
	path::{self, Path},
	return_error,
	symlink::Symlink,
	template::Template,
	Instance,
};

static CHECKOUTS_PATH: Lazy<Path> = Lazy::new(|| {
	Path::from_iter([
		path::Component::Normal(".tangram".to_string()),
		path::Component::Normal("checkouts".to_string()),
	])
});

impl Instance {
	pub async fn vendor(&self, artifact_hash: Hash) -> Result<Hash> {
		// Get the directory artifact.
		let artifact = self.get_artifact_local(artifact_hash)?;
		let Artifact::Directory(directory) = artifact else {
			return_error!("Cannot vendor a non-directory artifact.");
		};

		let mut vendored_directory = Directory::default();
		let mut checkout_artifacts = Vec::<Hash>::new();

		for (entry_name, &entry) in &directory.entries {
			// Get the path of this entry.
			let entry_path = Path::from_iter([path::Component::Normal(entry_name.clone())]);

			// Vendor the entry artifact.
			let vendored_hash = self
				.vendor_to(entry, &entry_path, &mut checkout_artifacts)
				.await?;

			// Add the vendored entry to the vendored directory.
			vendored_directory
				.entries
				.insert(entry_name.clone(), vendored_hash);
		}

		// Create a directory of checked out artifacts.
		let mut checkouts_directory = Directory {
			entries: BTreeMap::new(),
		};
		while let Some(artifact) = checkout_artifacts.pop() {
			// If we've already checked out this artifact, skip it.
			let checkouts_entry = checkouts_directory.entries.entry(artifact.to_string());
			let btree_map::Entry::Vacant(checkouts_entry) = checkouts_entry else {
				continue;
			};

			// Get the path for the checkout. It will be placed at `.tangram/checkouts/$HASH`.
			let checkout_artifact_path = CHECKOUTS_PATH
				.clone()
				.join([path::Component::Normal(artifact.to_string())]);

			// Vendor the artifact and place it at the checkout artifact path.
			let vendored_artifact = self
				.vendor_to(artifact, &checkout_artifact_path, &mut checkout_artifacts)
				.await?;
			checkouts_entry.insert(vendored_artifact);
		}

		// Create the `.tagnram` directory containing the checkouts directory.
		let checkouts_artifact_hash = self
			.add_artifact(&Artifact::Directory(checkouts_directory))
			.await?;
		let dot_tangram_artifact = Artifact::Directory(Directory {
			entries: BTreeMap::from_iter([("checkouts".to_string(), checkouts_artifact_hash)]),
		});
		let dot_tangram_artifact_hash = self.add_artifact(&dot_tangram_artifact).await?;

		// Add the `.tangram` directory to the root of the vendored directory.
		vendored_directory
			.entries
			.insert(".tangram".to_string(), dot_tangram_artifact_hash);

		// Return a hash for the vendored directory.
		let vendored_hash = self
			.add_artifact(&Artifact::Directory(vendored_directory))
			.await?;

		Ok(vendored_hash)
	}

	#[async_recursion]
	async fn vendor_to(
		&self,
		artifact_hash: Hash,
		artifact_path: &Path,
		checkout_artifacts: &mut Vec<Hash>,
	) -> Result<Hash> {
		let artifact = self.get_artifact_local(artifact_hash)?;

		let vendored_artifact = match artifact {
			Artifact::File(file) => {
				let File {
					blob_hash,
					executable,
					references,
				} = file;

				// Add all references to the list of artifacts to checkout.
				checkout_artifacts.extend(&references);

				// Strip the references from the file.
				let vendored_file = File {
					blob_hash,
					executable,
					references: vec![],
				};

				Artifact::File(vendored_file)
			},
			Artifact::Directory(directory) => {
				let mut vendored_directory = Directory::default();

				// Vendor each entry recursively.
				for (entry_name, &entry) in &directory.entries {
					let entry_path = artifact_path
						.clone()
						.join([path::Component::Normal(entry_name.clone())]);

					// Vendor the entry.
					let vendored_hash = self
						.vendor_to(entry, &entry_path, checkout_artifacts)
						.await?;

					// Add it to the vendored directory.
					vendored_directory
						.entries
						.insert(entry_name.clone(), vendored_hash);
				}

				Artifact::Directory(vendored_directory)
			},
			Artifact::Symlink(symlink) => {
				// Get the path of the directory containing the symlink.
				// NOTE: This works because paths are normalized, so pushing the parent directory will work even if the symlink's target is a file.
				let artifact_dir_path = artifact_path.clone().join([path::Component::ParentDir]);

				// Get the path to the checkouts directory relative to the symlink's directory.
				let checkouts_path = CHECKOUTS_PATH.diff(&artifact_dir_path);

				// Render the symlink target to refer to the checkouts directory.
				let vendored_target =
					vendored_render(&symlink.target, &checkouts_path, checkout_artifacts)?;
				let vendored_target = Template {
					components: vec![crate::template::Component::String(vendored_target)],
				};

				// Return a new symlink with the rendered target.
				Artifact::Symlink(Symlink {
					target: vendored_target,
				})
			},
		};

		// Add the vendored artifact.
		let vendored_hash = self.add_artifact(&vendored_artifact).await?;
		Ok(vendored_hash)
	}
}

fn vendored_render(
	template: &Template,
	checkouts_path: &Path,
	checkout_artifacts: &mut Vec<Hash>,
) -> Result<String> {
	let mut rendered = String::new();

	for component in &template.components {
		match component {
			crate::template::Component::String(value) => {
				rendered += value;
			},
			crate::template::Component::Artifact(artifact) => {
				checkout_artifacts.push(*artifact);

				let artifact_path = checkouts_path
					.clone()
					.join([path::Component::Normal(artifact.to_string())]);

				rendered += &artifact_path.to_string();
			},
			crate::template::Component::Placeholder(_) => {
				return_error!("Cannot vendor a symlink with placeholders.");
			},
		}
	}

	Ok(rendered)
}
