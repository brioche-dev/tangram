use super::cache::Cache;
use crate::{
	client::{Client, Transport},
	expression::{Artifact, Expression},
	hash::Hash,
	id::Id,
	server::{self, expression::AddExpressionOutcome},
};
use anyhow::{anyhow, bail, Context, Result};
use async_recursion::async_recursion;
use futures::future::try_join_all;
use std::{path::Path, sync::Arc};

impl Client {
	pub async fn checkin(&self, path: &Path) -> Result<Hash> {
		// Create a cache.
		let cache = Cache::new(path, Arc::clone(&self.file_system_semaphore));

		// Checkin the expression for the path.
		self.checkin_path(&cache, path).await?;

		// Retrieve the expression for the path.
		let (hash, _) = cache.get(path).await?.unwrap();

		// Add the artifact expression.
		let hash = self
			.add_expression(&Expression::Artifact(Artifact { hash }))
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
				self.add_blob(path, blob_hash).await?;
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

			AddExpressionOutcome::MissingExpressions { .. } => unreachable!(),
		};

		// Attempt to add the expression again. At this point, there should not be any missing entries or a missing blob.
		let outcome = self.try_add_expression(&expression).await?;
		if !matches!(outcome, AddExpressionOutcome::Added { .. }) {
			bail!("An unexpected error occurred.");
		}

		Ok(())
	}

	pub async fn add_blob(&self, path: &Path, hash: Hash) -> Result<Hash> {
		// Get the server path if it is local.
		let local_server_path = match &self.transport {
			Transport::InProcess(server) => Some(server.path()),
			Transport::Unix(unix) => Some(unix.path.as_ref()),
			Transport::Tcp(_) => None,
		};

		if let Some(local_server_path) = local_server_path {
			// If the server is local, copy the file to the server's blobs directory.
			tracing::trace!(r#"Copying file at path "{}"."#, path.display());

			// Create a temp path.
			let temp_path = std::env::temp_dir().join(Id::generate().to_string());

			// Copy the file to the temp path.
			let permit = self.file_system_semaphore.acquire().await.unwrap();
			tokio::fs::copy(&path, &temp_path).await?;
			drop(permit);

			// Move the temp file to the server's blobs directory.
			let permit = self.file_system_semaphore.acquire().await.unwrap();
			let blob_path = local_server_path.join("blobs").join(hash.to_string());
			tokio::fs::rename(&temp_path, &blob_path).await?;
			drop(permit);

			Ok(hash)
		} else if let Some(http) = self.transport.as_http() {
			// Create a stream for the file.
			let file = tokio::fs::File::open(&path)
				.await
				.with_context(|| format!(r#"Failed to open file at path "{}"."#, path.display()))?;
			let stream = tokio_util::io::ReaderStream::new(file);
			let request = hyper::Body::wrap_stream(stream);

			// Perform the request.
			let response = http.post(&format!("/blobs/{hash}"), request).await?;

			// Deserialize the response.
			let response = hyper::body::to_bytes(response)
				.await
				.context("Failed to read the response.")?;
			let response: server::blob::CreateResponse =
				serde_json::from_slice(&response).context("Failed to deserialize the response.")?;

			Ok(response.blob_hash)
		} else {
			unreachable!()
		}
	}
}
