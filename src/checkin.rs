use crate::{
	artifact::{AddArtifactOutcome, ArtifactHash},
	watcher::Watcher,
	Cli,
};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use futures::future::try_join_all;
use std::{path::Path, sync::Arc};

impl Cli {
	pub async fn checkin(&self, path: &Path) -> Result<ArtifactHash> {
		// Create a watcher.
		let watcher = Watcher::new(self.path(), Arc::clone(&self.inner.file_semaphore));

		// Check in the artifact for the path recursively.
		self.checkin_path(&watcher, path).await?;

		// Get the artifact for the path.
		let (artifact_hash, _) = watcher.get(path).await?.unwrap();

		Ok(artifact_hash)
	}

	#[async_recursion]
	async fn checkin_path(&self, watcher: &Watcher, path: &Path) -> Result<()> {
		// Get the artifact hash and artifact for the path, computing them if necessary.
		let (_, artifact) = watcher.get(path).await?.with_context(|| {
			let path = path.display();
			format!(r#"No file system object found at path "{path}"."#)
		})?;

		// Attempt to add the artifact.
		let outcome = self.try_add_artifact(&artifact).await?;

		// Handle the outcome.
		match outcome {
			// If the artifact was added, we are done.
			AddArtifactOutcome::Added { .. } => return Ok(()),

			// If the artifact is a directory and there were missing entries, recurse to add them.
			AddArtifactOutcome::DirectoryMissingEntries { entries } => {
				try_join_all(entries.into_iter().map(|(entry_name, _)| async {
					let path = path.join(entry_name);
					self.checkin_path(watcher, &path).await?;
					Ok::<_, anyhow::Error>(())
				}))
				.await?;
			},

			// If the artifact is a file and the blob is missing, add it.
			AddArtifactOutcome::FileMissingBlob { blob_hash } => {
				let permit = self.inner.file_semaphore.acquire().await?;

				let temp_path = self.temp_path();
				let blob_path = self.blob_path(blob_hash);

				// Copy to the temp path.
				tokio::fs::copy(path, &temp_path).await?;

				// Rename from the temp path to the blob path.
				tokio::fs::rename(&temp_path, &blob_path).await?;

				drop(permit);
			},

			// If the artifact is a dependency that is missing, check it in.
			AddArtifactOutcome::DependencyMissing { .. } => {
				// Read the target from the path.
				let permit = self.inner.file_semaphore.acquire().await.unwrap();
				let target = tokio::fs::read_link(path).await?;
				drop(permit);

				// Checkin the path pointed to by the symlink.
				self.checkin_path(watcher, &path.join(target)).await?;
			},
		};

		// Attempt to add the artifact again. At this point, there should not be any missing entries or a missing blob.
		let outcome = self.try_add_artifact(&artifact).await?;
		if !matches!(outcome, AddArtifactOutcome::Added { .. }) {
			bail!("An unexpected error occurred.");
		}

		Ok(())
	}
}
