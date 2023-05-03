use super::Artifact;
use crate::{
	directory::{Directory, self},
	error::{return_error, Error, Result},
	file::File,
	instance::Instance,
	path::{self, Path},
	symlink::Symlink,
	template,
};
use async_recursion::async_recursion;
use futures::{stream::FuturesOrdered, TryStreamExt};
use once_cell::sync::Lazy;
use std::collections::HashSet;

static TANGRAM_ARTIFACTS_PATH: Lazy<Path> = Lazy::new(|| Path::new(".tangram/artifacts"));

impl Artifact {
	/// Bundle an artifact with all of its recursive references at `.tangram/artifacts`.
	pub async fn bundle(&self, tg: &Instance) -> Result<Artifact> {
		// Collect the recursive references.
		let mut references = HashSet::default();
		self.collect_recursive_references(tg, &mut references)
			.await?;

		// If there are no references, then return the artifact.
		if references.is_empty() {
			return Ok(self.clone());
		}

		// Bundle the artifact, stripping any references recursively.
		let artifact = self.bundle_inner(tg, &Path::empty()).await?;

		// Add the bundled artifact at the correct path.
		let artifact = match artifact {
			// If the artifact is a directory, bundle it to the empty path.
			Artifact::Directory(artifact) => artifact,

			// If the artifact is an executable file, bundle it at .tangram/run.
			Artifact::File(artifact) if artifact.executable() => {
				directory::Builder::new()
					.add(tg, ".tangram/run", artifact)
					.await?
					.build(tg)
					.await?
			},

			// Return an error otherwise.
			_ => return_error!("The artifact must be a directory or an executable file."),
		};

		// Create the references directory by bundling each reference at `.tangram/artifacts/HASH`.
		let entries = references
			.into_iter()
			.map(|reference| {
				async move {
					// Create the path for the reference at `.tangram/artifacts/HASH`.
					let path = TANGRAM_ARTIFACTS_PATH
						.clone()
						.join(path::Component::Normal(reference.hash().to_string()));

					// Bundle the reference.
					let artifact = reference.bundle_inner(tg, &path).await?;

					Ok::<_, Error>((reference.hash().to_string(), artifact))
				}
			})
			.collect::<FuturesOrdered<_>>()
			.try_collect()
			.await?;

		let directory = Directory::new(tg, entries).await?;

		// Add the references directory to the artifact at `.tangram/artifacts`.
		let artifact = artifact
			.builder(tg)
			.await?
			.add(tg, TANGRAM_ARTIFACTS_PATH.clone(), directory)
			.await?
			.build(tg)
			.await?
			.into();

		Ok(artifact)
	}

	/// Remove all references from an artifact recursively, rendering symlink targets to a relative path from `path` to `.tangram/artifacts/HASH`.
	#[async_recursion]
	async fn bundle_inner(&self, tg: &'async_recursion Instance, path: &Path) -> Result<Artifact> {
		match self {
			// If the artifact is a directory, then recurse to bundle its entries.
			Artifact::Directory(directory) => {
				let entries = directory
					.entries(tg)
					.await?
					.into_iter()
					.map(|(name, artifact)| {
						async move {
							// Create the path for the entry.
							let path = path.clone().join(&name);

							// Bundle the entry.
							let artifact = artifact.bundle_inner(tg, &path).await?;

							Ok::<_, Error>((name, artifact))
						}
					})
					.collect::<FuturesOrdered<_>>()
					.try_collect()
					.await?;

				Ok(Artifact::Directory(Directory::new(tg, entries).await?))
			},

			// If the artifact is a file, then return the file without any references.
			Artifact::File(file) => Ok(Artifact::File(
				File::new(tg, file.blob(), file.executable(), &[]).await?,
			)),

			// If the artifact is a symlink, then return the symlink with its target rendered with artifacts pointing to `.tangram/artifacts/HASH`.
			Artifact::Symlink(symlink) => {
				// Render the target.
				let target = symlink
					.target()
					.render(|component| async move {
						match component {
							// Render a string component as is.
							template::Component::String(string) => Ok(string.into()),

							// Render an artifact component with the diff from the path's parent to the referenced artifact's bundled path.
							template::Component::Artifact(artifact) => {
								let bundle_path = TANGRAM_ARTIFACTS_PATH
									.clone()
									.join(path::Component::Normal(artifact.hash().to_string()));
								let path = bundle_path
									.diff(&path.clone().join(path::Component::Parent))
									.to_string()
									.into();
								Ok(path)
							},

							// Placeholder components are not allowed.
							template::Component::Placeholder(_) => {
								return_error!(
									"Cannot bundle a symlink whose target has placeholders."
								);
							},
						}
					})
					.await?
					.into();

				Ok(Artifact::Symlink(Symlink::new(tg, target).await?))
			},
		}
	}
}
