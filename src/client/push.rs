use super::Client;
use crate::{artifact::Artifact, error::Result, instance::Instance};
use async_recursion::async_recursion;

impl Client {
	/// Push an artifact to a remote server.
	#[async_recursion]
	#[must_use]
	pub async fn push(&self, _tg: &Instance, _artifact: &Artifact) -> Result<()> {
		// // Get the artifact.
		// let artifact = self.get_artifact_local(artifact_hash)?;

		// // Try to add the artifact.
		// let outcome = client.try_add_artifact(&artifact).await?;

		// // Handle the outcome.
		// match outcome {
		// 	// If the artifact was added, then we are done.
		// 	artifact::add::Outcome::Added { .. } => return Ok(()),

		// 	// If there were missing entries, then recurse to push them.
		// 	artifact::add::Outcome::MissingEntries { entries } => {
		// 		try_join_all(entries.into_iter().map(|(_, hash)| async move {
		// 			self.push(client, hash).await?;
		// 			Ok::<_, Error>(())
		// 		}))
		// 		.await?;
		// 	},

		// 	// If the blob is missing, then push it.
		// 	artifact::add::Outcome::MissingBlob { blob_hash } => {
		// 		// Get the blob.
		// 		let blob = self
		// 			.get_blob(blob_hash)
		// 			.await
		// 			.wrap_err("Failed to get the blob.")?;

		// 		// Add the blob.
		// 		client
		// 			.add_blob(blob, blob_hash)
		// 			.await
		// 			.wrap_err("Failed to add the blob.")?;
		// 	},

		// 	// If there are missing references, then recurse to push them.
		// 	artifact::add::Outcome::MissingReferences { artifact_hashes } => {
		// 		try_join_all(artifact_hashes.into_iter().map(|hash| async move {
		// 			self.push(client, hash).await?;
		// 			Ok::<_, Error>(())
		// 		}))
		// 		.await?;
		// 	},
		// };

		// // Attempt to push the artifact again. At this point, there should not be any missing entries or a missing blob.
		// let outcome = client.try_add_artifact(&artifact).await?;
		// if !matches!(outcome, artifact::add::Outcome::Added { .. }) {
		// 	return_error!("An unexpected error occurred.");
		// }

		Ok(())
	}
}
