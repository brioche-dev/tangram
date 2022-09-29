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
		let path = format!("/expressions/{hash}");
		let expression = self.get_expression(hash).await?;
		let outcome: AddExpressionOutcome = client.post_json(&path, &expression).await?;
		Ok(outcome)
	}

	pub async fn push_blob(&self, hash: Hash, client: &Client) -> Result<Hash> {
		// Create a stream from the blob.
		let blob_path = self.blob_path(hash);
		let blob = tokio::fs::File::open(&blob_path).await.with_context(|| {
			format!(r#"Failed to open blob at path "{}"."#, blob_path.display())
		})?;
		let stream = tokio_util::io::ReaderStream::new(blob);
		let request = hyper::Body::wrap_stream(stream);

		// Perform the request.
		let response = client.post(&format!("/blobs/{hash}"), request).await?;

		// Read the response.
		let response = hyper::body::to_bytes(response)
			.await
			.context("Failed to read the response.")?;
		let hash = String::from_utf8(response.to_vec())
			.context("Failed to read the response as UTF-8.")?
			.parse()
			.context("Failed to parse the hash.")?;

		Ok(hash)
	}
}
