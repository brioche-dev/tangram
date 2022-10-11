use crate::{builder::Shared, expression::AddExpressionOutcome, hash::Hash};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use futures::future::try_join_all;

impl Shared {
	/// Push an expression to a remote server.
	#[async_recursion]
	#[must_use]
	pub async fn push(&self, hash: Hash) -> Result<()> {
		// Get the expression.
		let expression = self.get_expression_local(hash)?;

		// Try to add the expression.
		let outcome = self
			.expression_client
			.try_add_expression(&expression)
			.await?;

		// Handle the outcome.
		match outcome {
			// If the expression was added, we are done.
			AddExpressionOutcome::Added { .. } => return Ok(()),

			// If this expression is a directory and there were missing entries, recurse to push them.
			AddExpressionOutcome::DirectoryMissingEntries { entries } => {
				try_join_all(entries.into_iter().map(|(_, hash)| async move {
					self.push(hash).await?;
					Ok::<_, anyhow::Error>(())
				}))
				.await?;
			},

			// If this expression is a file and the blob is missing, push it.
			AddExpressionOutcome::FileMissingBlob { blob_hash } => {
				// Get the path to the blob.
				let blob_path = self.blob_path(blob_hash);

				// Create a stream for the file.
				let file =
					Box::new(tokio::fs::File::open(&blob_path).await.with_context(|| {
						format!(r#"Failed to open file at path "{}"."#, blob_path.display())
					})?);

				// Add the blob.
				self.blob_client.add_blob(file, blob_hash).await?;
			},

			// If this expression is a dependency that is missing, push it.
			AddExpressionOutcome::DependencyMissing { hash } => {
				self.push(hash).await?;
			},

			// If this expression has missing subexpressions, push them.
			AddExpressionOutcome::MissingExpressions { hashes } => {
				for hash in hashes {
					self.push(hash).await?;
				}
			},
		};

		// Attempt to push the expression again. At this point, there should not be any missing entries or a missing blob.
		let outcome = self
			.expression_client
			.try_add_expression(&expression)
			.await?;
		if !matches!(outcome, AddExpressionOutcome::Added { .. }) {
			bail!("An unexpected error occurred.");
		}

		Ok(())
	}
}
