use crate::{builder::Shared, expression::AddExpressionOutcome, hash::Hash};
use anyhow::{bail, Result};
use async_recursion::async_recursion;
use futures::future::try_join_all;

impl Shared {
	/// Pull an expression from a remote server.
	#[async_recursion]
	#[must_use]
	pub async fn pull(&self, hash: Hash) -> Result<()> {
		// Get the expression.
		let expression = self.client.get_expression(hash).await?;

		// Try to add the expression.
		let outcome = self.try_add_expression(&expression).await?;

		// Handle the outcome.
		match outcome {
			AddExpressionOutcome::Added { .. } => return Ok(()),
			AddExpressionOutcome::DirectoryMissingEntries { entries } => {
				// Pull the missing entries.
				try_join_all(entries.into_iter().map(|(_, hash)| async move {
					self.pull(hash).await?;
					Ok::<_, anyhow::Error>(())
				}))
				.await?;
			},
			AddExpressionOutcome::FileMissingBlob { blob_hash } => {
				// Pull the blob.
				self.client.get_blob(blob_hash).await?;
			},
			AddExpressionOutcome::DependencyMissing { hash } => {
				// Pull the missing dependency.
				self.pull(hash).await?;
			},
			AddExpressionOutcome::MissingExpressions { hashes } => {
				try_join_all(hashes.into_iter().map(|hash| async move {
					self.pull(hash).await?;
					Ok::<_, anyhow::Error>(())
				}))
				.await?;
			},
		};

		// Attempt to add the expression again. At this point, there should not be any missing entries or a missing blob.
		let outcome = self.try_add_expression(&expression).await?;
		if !matches!(outcome, AddExpressionOutcome::Added { .. }) {
			bail!("An unexpected error occurred.");
		}

		Ok(())
	}
}
