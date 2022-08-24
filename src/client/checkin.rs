use super::object_cache::ObjectCache;
use crate::{
	artifact::Artifact,
	client::{Client, Transport},
	object::BlobHash,
	server::{object::AddObjectOutcome, Server},
};
use anyhow::{anyhow, bail, Result};
use async_recursion::async_recursion;
use std::{path::Path, sync::Arc};

impl Client {
	pub async fn checkin(&self, path: &Path) -> Result<Artifact> {
		let object_cache = ObjectCache::new(path, Arc::clone(&self.file_system_semaphore));
		self.checkin_object_for_path(&object_cache, path).await?;
		let (object_hash, _) = object_cache.get(path).await?.unwrap();
		let artifact = self.create_artifact(object_hash).await?;
		Ok(artifact)
	}

	#[async_recursion]
	async fn checkin_object_for_path(&self, object_cache: &ObjectCache, path: &Path) -> Result<()> {
		tracing::trace!(r#"Checking in object at path "{}"."#, path.display());

		// Retrieve the object hash and object for the path or compute them if necessary.
		let (_, object) = object_cache.get(path).await?.ok_or_else(|| {
			anyhow!(
				r#"No file system object found at path "{}"."#,
				path.display(),
			)
		})?;

		// Attempt to add the object.
		let outcome = match &self.transport {
			Transport::InProcess { server, .. } => server.add_object(&object).await?,
			_ => unimplemented!(),
		};

		// Handle the outcome.
		match outcome {
			// If the object was added, we are done.
			AddObjectOutcome::Added(_) => return Ok(()),

			// If this object is a directory and there were missing entries, recurse to add them.
			AddObjectOutcome::DirectoryMissingEntries(entries) => {
				futures::future::try_join_all(entries.into_iter().map(|(entry_name, _)| {
					async {
						let path = path.join(entry_name);
						self.checkin_object_for_path(object_cache, &path).await?;
						Ok::<_, anyhow::Error>(())
					}
				}))
				.await?;
			},

			// If this object is a file and the blob is missing, add it.
			AddObjectOutcome::FileMissingBlob(blob_hash) => {
				match &self.transport {
					Transport::InProcess { server, .. } => {
						self.add_blob_for_path(server, path, blob_hash).await?;
					},
					_ => unimplemented!(),
				};
			},

			// If this object is a dependency that is missing, check it in.
			AddObjectOutcome::DependencyMissing(_object_hash) => {
				// Read the target from the path.
				let permit = self.file_system_semaphore.acquire().await.unwrap();
				let target = tokio::fs::read_link(path).await?;
				drop(permit);

				// Checkin the path pointed to by the symlink.
				self.checkin_object_for_path(object_cache, &path.join(target))
					.await?;
			},
		};

		// Attempt to add the object again. At this point, there should not be any missing entries or a missing blob.
		let outcome = match &self.transport {
			Transport::InProcess { server, .. } => server.add_object(&object).await?,
			_ => unimplemented!(),
		};
		if !matches!(outcome, AddObjectOutcome::Added(_)) {
			bail!("An unexpected error occurred.");
		}

		Ok(())
	}

	pub async fn add_blob_for_path(
		&self,
		server: &Arc<Server>,
		path: &Path,
		blob_hash: BlobHash,
	) -> Result<BlobHash> {
		tracing::trace!(r#"Copying file at path "{}"."#, path.display());

		// Create a temp.
		let temp = server.create_temp().await?;
		let temp_path = server.temp_path(&temp);

		// Copy the file to the temp.
		let permit = self.file_system_semaphore.acquire().await.unwrap();
		tokio::fs::copy(&path, &temp_path).await?;
		drop(permit);

		// Move the temp file to the server's blobs directory.
		let permit = self.file_system_semaphore.acquire().await.unwrap();
		let blob_path = server.blob_path(blob_hash);
		tokio::fs::rename(&temp_path, &blob_path).await?;
		drop(permit);

		Ok(blob_hash)
	}
}
