use super::object_cache::ObjectCache;
use crate::{
	artifact::Artifact,
	client::{Client, Transport},
	id::Id,
	object::{BlobHash, Object, ObjectHash},
	server::{self, object::AddObjectOutcome},
};
use anyhow::{anyhow, bail, Context, Result};
use async_recursion::async_recursion;
use std::{path::Path, sync::Arc};

impl Client {
	pub async fn checkin(&self, path: &Path) -> Result<Artifact> {
		// Create an object cache.
		let object_cache = ObjectCache::new(path, Arc::clone(&self.file_system_semaphore));

		// Checkin the object for the path.
		self.checkin_object_for_path(&object_cache, path).await?;

		// Retrieve the object for the path.
		let (object_hash, _) = object_cache.get(path).await?.unwrap();

		// Create an artifact for the root object.
		let artifact = self.create_artifact(object_hash).await?;

		Ok(artifact)
	}

	#[async_recursion]
	async fn checkin_object_for_path(&self, object_cache: &ObjectCache, path: &Path) -> Result<()> {
		tracing::trace!(r#"Checking in object at path "{}"."#, path.display());

		// Retrieve the object hash and object for the path, computing them if necessary.
		let (object_hash, object) = object_cache.get(path).await?.ok_or_else(|| {
			anyhow!(
				r#"No file system object found at path "{}"."#,
				path.display(),
			)
		})?;

		// Attempt to add the object.
		let outcome = self.add_object(object_hash, &object).await?;

		// Handle the outcome.
		match outcome {
			// If the object was added, we are done.
			AddObjectOutcome::Added { .. } => return Ok(()),

			// If this object is a directory and there were missing entries, recurse to add them.
			AddObjectOutcome::DirectoryMissingEntries { entries } => {
				futures::future::try_join_all(entries.into_iter().map(|(entry_name, _)| async {
					let path = path.join(entry_name);
					self.checkin_object_for_path(object_cache, &path).await?;
					Ok::<_, anyhow::Error>(())
				}))
				.await?;
			},

			// If this object is a file and the blob is missing, add it.
			AddObjectOutcome::FileMissingBlob { blob_hash } => {
				self.add_blob(path, blob_hash).await?;
			},

			// If this object is a dependency that is missing, check it in.
			AddObjectOutcome::DependencyMissing { .. } => {
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
		let outcome = self.add_object(object_hash, &object).await?;
		if !matches!(outcome, AddObjectOutcome::Added { .. }) {
			bail!("An unexpected error occurred.");
		}

		Ok(())
	}

	pub async fn add_object(
		&self,
		object_hash: ObjectHash,
		object: &Object,
	) -> Result<AddObjectOutcome> {
		match self.transport.as_in_process_or_http() {
			super::transport::InProcessOrHttp::InProcess(server) => {
				let outcome = server.add_object(object_hash, object).await?;
				Ok(outcome)
			},
			super::transport::InProcessOrHttp::Http(http) => {
				let path = format!("/objects/{object_hash}");
				let outcome = http.post_json(&path, object).await?;
				Ok(outcome)
			},
		}
	}

	pub async fn add_blob(&self, path: &Path, hash: BlobHash) -> Result<BlobHash> {
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
