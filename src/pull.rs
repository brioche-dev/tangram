use crate::{artifact::AddArtifactOutcome, artifact::ArtifactHash, client::Client, Cli};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use futures::future::try_join_all;

impl Cli {
	/// Pull an artifact from a remote server.
	#[async_recursion]
	#[must_use]
	pub async fn pull(&self, client: &Client, artifact_hash: ArtifactHash) -> Result<()> {
		// Get the artifact.
		let artifact = client
			.try_get_artifact(artifact_hash)
			.await?
			.with_context(|| format!(r#"Unable to find artifact with hash "{artifact_hash}""#))?;

		// Try to add the artifact.
		let outcome = self.try_add_artifact(&artifact).await?;

		// Handle the outcome.
		match outcome {
			AddArtifactOutcome::Added { .. } => return Ok(()),
			AddArtifactOutcome::DirectoryMissingEntries { entries } => {
				// Pull the missing entries.
				try_join_all(entries.into_iter().map(|(_, artifact_hash)| async move {
					self.pull(client, artifact_hash).await?;
					Ok::<_, anyhow::Error>(())
				}))
				.await?;
			},
			AddArtifactOutcome::FileMissingBlob { blob_hash } => {
				// Pull the blob.
				client.get_blob(blob_hash).await?;
			},
			AddArtifactOutcome::DependencyMissing { artifact_hash } => {
				// Pull the missing dependency.
				self.pull(client, artifact_hash).await?;
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
