use crate::{
	artifact,
	client::Client,
	error::{return_error, Error, Result, WrapErr},
	Instance,
};
use async_recursion::async_recursion;
use futures::future::try_join_all;

impl Instance {
	/// Push an artifact to a remote server.
	#[async_recursion]
	#[must_use]
	pub async fn push(&self, client: &Client, artifact_hash: artifact::Hash) -> Result<()> {
		// Get the artifact.
		let artifact = self.get_artifact_local(artifact_hash)?;

		// Try to add the artifact.
		let outcome = client.try_add_artifact(&artifact).await?;

		// Handle the outcome.
		match outcome {
			// If the artifact was added, then we are done.
			artifact::add::Outcome::Added { .. } => return Ok(()),

			// If this artifact is a directory and there were missing entries, then recurse to push them.
			artifact::add::Outcome::DirectoryMissingEntries { entries } => {
				try_join_all(entries.into_iter().map(|(_, hash)| async move {
					self.push(client, hash).await?;
					Ok::<_, Error>(())
				}))
				.await?;
			},

			// If this artifact is a file and the blob is missing, then push it.
			artifact::add::Outcome::FileMissingBlob { blob_hash } => {
				let _permit = self.file_semaphore.acquire().await.map_err(Error::other)?;

				// Get the blob.
				let blob = self
					.get_blob(blob_hash)
					.await
					.wrap_err("Failed to get the blob.")?;

				// Add the blob.
				client
					.add_blob(blob, blob_hash)
					.await
					.wrap_err("Failed to add the blob.")?;
			},

			// If this artifact is a reference whose artifact is missing, then push it.
			artifact::add::Outcome::ReferenceMissingArtifact { artifact_hash } => {
				self.push(client, artifact_hash).await?;
			},
		};

		// Attempt to push the artifact again. At this point, there should not be any missing entries or a missing blob.
		let outcome = client.try_add_artifact(&artifact).await?;
		if !matches!(outcome, artifact::add::Outcome::Added { .. }) {
			return_error!("An unexpected error occurred.");
		}

		Ok(())
	}
}
