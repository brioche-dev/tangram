use super::Client;
use crate::{artifact::Artifact, error::Result, instance::Instance};
use async_recursion::async_recursion;

impl Client {
	/// Pull an artifact.
	#[async_recursion]
	#[must_use]
	pub async fn pull(&self, _tg: &Instance, _artifact: &Artifact) -> Result<()> {
		// // Get the artifact.
		// let artifact = client
		// 	.try_get_artifact(artifact_hash)
		// 	.await?
		// 	.wrap_err_with(|| format!(r#"Unable to find artifact with hash "{artifact_hash}""#))?;

		// // Try to add the artifact.
		// let outcome = self.try_add_artifact(&artifact).await?;

		// // Handle the outcome.
		// match outcome {
		// 	artifact::add::Outcome::Added { .. } => return Ok(()),

		// 	artifact::add::Outcome::MissingEntries { entries } => {
		// 		// Pull the missing entries.
		// 		try_join_all(entries.into_iter().map(|(_, artifact_hash)| async move {
		// 			self.pull(client, artifact_hash).await?;
		// 			Ok::<_, Error>(())
		// 		}))
		// 		.await?;
		// 	},

		// 	artifact::add::Outcome::MissingBlob { blob_hash } => {
		// 		// Pull the blob.
		// 		let blob = client.get_blob(blob_hash).await?;
		// 		self.add_blob(blob).await?;
		// 	},

		// 	artifact::add::Outcome::MissingReferences { artifact_hashes } => {
		// 		// Pull the missing references.
		// 		try_join_all(artifact_hashes.into_iter().map(|artifact_hash| async move {
		// 			self.pull(client, artifact_hash).await?;
		// 			Ok::<_, Error>(())
		// 		}))
		// 		.await?;
		// 	},
		// };

		// // Attempt to add the artifact again. At this point, there should not be any missing entries, blobs, or references.
		// let outcome = self.try_add_artifact(&artifact).await?;
		// if !matches!(outcome, artifact::add::Outcome::Added { .. }) {
		// 	return Err(Error::message("An unexpected error occurred."));
		// }

		Ok(())
	}
}
