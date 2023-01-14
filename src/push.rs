use crate::{
	artifact::{AddArtifactOutcome, ArtifactHash},
	client::Client,
	Cli,
};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use futures::future::try_join_all;

impl Cli {
	/// Push an artifact to a remote server.
	#[async_recursion]
	#[must_use]
	pub async fn push(&self, client: &Client, artifact_hash: ArtifactHash) -> Result<()> {
		// Get the artifact.
		let artifact = self.get_artifact_local(artifact_hash)?;

		// Try to add the artifact.
		let outcome = client.try_add_artifact(&artifact).await?;

		// Handle the outcome.
		match outcome {
			// If the artifact was added, we are done.
			AddArtifactOutcome::Added { .. } => return Ok(()),

			// If this artifact is a directory and there were missing entries, recurse to push them.
			AddArtifactOutcome::DirectoryMissingEntries { entries } => {
				try_join_all(entries.into_iter().map(|(_, hash)| async move {
					self.push(client, hash).await?;
					Ok::<_, anyhow::Error>(())
				}))
				.await?;
			},

			// If this artifact is a file and the blob is missing, push it.
			AddArtifactOutcome::FileMissingBlob { blob_hash } => {
				let _permit = self.inner.file_system_semaphore.acquire().await?;

				// Get the blob.
				let blob = self
					.get_blob(blob_hash)
					.await
					.context("Failed to get the blob.")?;

				// Add the blob.
				client
					.add_blob(blob, blob_hash)
					.await
					.context("Failed to add the blob.")?;
			},

			// If this artifact is a dependency that is missing, push it.
			AddArtifactOutcome::DependencyMissing { artifact_hash } => {
				self.push(client, artifact_hash).await?;
			},
		};

		// Attempt to push the artifact again. At this point, there should not be any missing entries or a missing blob.
		let outcome = client.try_add_artifact(&artifact).await?;
		if !matches!(outcome, AddArtifactOutcome::Added { .. }) {
			bail!("An unexpected error occurred.");
		}

		Ok(())
	}
}
