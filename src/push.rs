use crate::{
	artifact::{AddArtifactOutcome, ArtifactHash},
	client::Client,
	State,
};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use futures::future::try_join_all;

impl State {
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
				// Get the path to the blob.
				let blob_path = self.blob_path(blob_hash);

				// Create a stream for the file.
				let file =
					Box::new(tokio::fs::File::open(&blob_path).await.with_context(|| {
						format!(r#"Failed to open file at path "{}"."#, blob_path.display())
					})?);

				// Add the blob.
				client.add_blob(file, blob_hash).await?;
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
