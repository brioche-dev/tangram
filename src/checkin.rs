use crate::{
	builder,
	cache::Cache,
	expression::AddExpressionOutcome,
	expression::{Artifact, Expression},
	hash::Hash,
};
use anyhow::{anyhow, bail, Result};
use async_recursion::async_recursion;
use futures::future::try_join_all;
use std::{path::Path, sync::Arc};

impl builder::Shared {
	pub async fn checkin(&self, path: &Path) -> Result<Hash> {
		// Create a cache.
		let cache = Cache::new(path, Arc::clone(&self.file_system_semaphore));

		// Checkin the expression for the path.
		self.checkin_path(&cache, path).await?;

		// Retrieve the expression for the path.
		let (hash, _) = cache.get(path).await?.unwrap();

		// Add the artifact expression.
		let hash = self
			.add_expression(&Expression::Artifact(Artifact { root: hash }))
			.await?;

		Ok(hash)
	}

	#[async_recursion]
	async fn checkin_path(&self, cache: &Cache, path: &Path) -> Result<()> {
		tracing::trace!(r#"Checking in expression at path "{}"."#, path.display());

		// Retrieve the expression hash and expression for the path, computing them if necessary.
		let (_, expression) = cache.get(path).await?.ok_or_else(|| {
			anyhow!(
				r#"No file system object found at path "{}"."#,
				path.display(),
			)
		})?;

		// Attempt to add the expression.
		let outcome = self.try_add_expression(&expression).await?;

		// Handle the outcome.
		match outcome {
			// If the expression was added, we are done.
			AddExpressionOutcome::Added { .. } => return Ok(()),

			// If this expression is a directory and there were missing entries, recurse to add them.
			AddExpressionOutcome::DirectoryMissingEntries { entries } => {
				try_join_all(entries.into_iter().map(|(entry_name, _)| async {
					let path = path.join(entry_name);
					self.checkin_path(cache, &path).await?;
					Ok::<_, anyhow::Error>(())
				}))
				.await?;
			},

			// If this expression is a file and the blob is missing, add it.
			AddExpressionOutcome::FileMissingBlob { blob_hash } => {
				let blob_path = self.blob_path(blob_hash);
				tokio::fs::copy(path, blob_path).await?;
			},

			// If this expression is a dependency that is missing, check it in.
			AddExpressionOutcome::DependencyMissing { .. } => {
				// Read the target from the path.
				let permit = self.file_system_semaphore.acquire().await.unwrap();
				let target = tokio::fs::read_link(path).await?;
				drop(permit);

				// Checkin the path pointed to by the symlink.
				self.checkin_path(cache, &path.join(target)).await?;
			},

			AddExpressionOutcome::MissingExpressions { .. } => {
				bail!("Unexpected missing expressions during checkin.");
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
