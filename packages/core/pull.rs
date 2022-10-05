use crate::{
	builder,
	client::Client,
	expression::{AddExpressionOutcome, Expression},
	hash::Hash,
};
use anyhow::{bail, Result};
use async_recursion::async_recursion;
use futures::future::try_join_all;

impl builder::Shared {
	/// Pull an expression from a remote server.
	#[async_recursion]
	#[must_use]
	pub async fn pull(&self, hash: Hash, client: &Client) -> Result<()> {
		let expression = client.get_expression(hash).await?;
		let outcome = self.try_add_expression(&expression).await?;
		match outcome {
			AddExpressionOutcome::Added { .. } => return Ok(()),
			AddExpressionOutcome::DirectoryMissingEntries { entries } => {
				// Pull the missing entries.
				try_join_all(entries.into_iter().map(|(_, hash)| async move {
					self.pull(hash, client).await?;
					Ok::<_, anyhow::Error>(())
				}))
				.await?;
			},
			AddExpressionOutcome::FileMissingBlob { blob_hash } => {
				// Pull the blob.
				client.get_blob(blob_hash).await?;
			},
			AddExpressionOutcome::DependencyMissing { hash } => {
				// Pull the missing dependency.
				self.pull(hash, client).await?;
			},
			AddExpressionOutcome::MissingExpressions { hashes } => {
				try_join_all(hashes.into_iter().map(|hash| async move {
					self.pull(hash, client).await?;
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

	pub async fn pull_expression(&self, hash: Hash, client: &Client) -> Result<Expression> {
		let expression = client.get_expression(hash).await?;
		Ok(expression)
	}

	pub async fn pull_blob(&self, hash: Hash, client: &Client) -> Result<()> {
		// Get the blob.
		let response = client.get_blob(hash).await?;
		let mut body = match response {
			crate::blob::Blob::Local(_) => unreachable!(),
			crate::blob::Blob::Remote(body) => body,
		};

		// Create the file to write to.
		let blob_path = self.blob_path(hash);

		// Create a temp path to checkout the artifact to.
		let temp_path = self.create_temp_path();
		let mut temp_file = tokio::fs::File::create(&temp_path).await?;

		// Copy from the body to the file.
		tokio::io::copy(&mut body, &mut temp_file).await?;

		// Move the blob from the temp to the blobs directory.
		tokio::fs::rename(&temp_path, &blob_path).await?;

		Ok(())
	}
}
