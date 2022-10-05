use crate::{builder, client::Client, expression::AddExpressionOutcome, hash::Hash};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use futures::future::try_join_all;

impl builder::Shared {
	/// Push an expression to a remote server.
	#[async_recursion]
	#[must_use]
	pub async fn push(&self, hash: Hash, client: &Client) -> Result<()> {
		let outcome = self.try_push_expression(hash, client).await?;

		// Handle the outcome.
		match outcome {
			// If the expression was added, we are done.
			AddExpressionOutcome::Added { .. } => return Ok(()),

			// If this expression is a directory and there were missing entries, recurse to push them.
			AddExpressionOutcome::DirectoryMissingEntries { entries } => {
				try_join_all(entries.into_iter().map(|(_, hash)| async move {
					self.push(hash, client).await?;
					Ok::<_, anyhow::Error>(())
				}))
				.await?;
			},

			// If this expression is a file and the blob is missing, push it.
			AddExpressionOutcome::FileMissingBlob { blob_hash } => {
				self.push_blob(blob_hash, client).await?;
			},

			// If this expression is a dependency that is missing, push it.
			AddExpressionOutcome::DependencyMissing { hash } => {
				self.push(hash, client).await?;
			},

			// If this expression has missing subexpressions, push them.
			AddExpressionOutcome::MissingExpressions { hashes } => {
				for hash in hashes {
					self.push(hash, client).await?;
				}
			},
		};

		// Attempt to push the expression again. At this point, there should not be any missing entries or a missing blob.
		let outcome = self.try_push_expression(hash, client).await?;
		if !matches!(outcome, AddExpressionOutcome::Added { .. }) {
			bail!("An unexpected error occurred.");
		}

		Ok(())
	}

	pub async fn try_push_expression(
		&self,
		hash: Hash,
		client: &Client,
	) -> Result<AddExpressionOutcome> {
		let expression = self.get_expression(hash).await?;
		let outcome: AddExpressionOutcome = client.try_add_expression(&expression).await?;
		Ok(outcome)
	}

	pub async fn push_blob(&self, hash: Hash, client: &Client) -> Result<Hash> {
		// Create a stream from the blob.
		let blob_path = self.blob_path(hash);

		// Create a stream for the file.
		let file = tokio::fs::File::open(&blob_path).await.with_context(|| {
			format!(r#"Failed to open file at path "{}"."#, blob_path.display())
		})?;

		// Send the request.
		let hash = client.add_blob(Box::new(file), hash).await?;

		Ok(hash)
	}
}
