pub use self::hash::BlobHash;
use crate::{hash::Hasher, util::path_exists, State};
use anyhow::{Context, Result};
use tokio::io::{AsyncRead, AsyncWriteExt};
use tokio_stream::StreamExt;

mod hash;

pub type Blob = Box<dyn AsyncRead + Unpin + Send + Sync>;

impl State {
	pub async fn add_blob(&self, reader: impl AsyncRead + Unpin) -> Result<BlobHash> {
		// Get a file system permit.
		let permit = self.file_system_semaphore.acquire().await.unwrap();

		// Create a temp file to read the blob into.
		let temp_path = self.create_temp_path();
		let mut temp_file = tokio::fs::File::create(&temp_path).await?;

		// Compute the hash of the bytes in the reader and write the bytes to the temp file.
		let mut stream = tokio_util::io::ReaderStream::new(reader);
		let mut hasher = Hasher::new();
		while let Some(chunk) = stream.next().await {
			let chunk = chunk?;
			hasher.update(&chunk);
			temp_file.write_all(&chunk).await?;
		}
		let blob_hash = BlobHash(hasher.finalize());

		// Close the temp file.
		temp_file.sync_all().await?;
		drop(temp_file);

		// Move the temp file to the blobs path.
		let blob_path = self.blob_path(blob_hash);
		tokio::fs::rename(&temp_path, &blob_path).await?;

		// Drop the file system permit.
		drop(permit);

		Ok(blob_hash)
	}

	pub async fn get_blob(&self, blob_hash: BlobHash) -> Result<tokio::fs::File> {
		let blob = self
			.try_get_blob(blob_hash)
			.await?
			.with_context(|| format!(r#"Failed to get blob with hash "{blob_hash}"."#))?;
		Ok(blob)
	}

	pub async fn try_get_blob(&self, blob_hash: BlobHash) -> Result<Option<tokio::fs::File>> {
		// Get the blob path.
		let path = self.blob_path(blob_hash);

		// Check if the blob exists.
		if !path_exists(&path).await? {
			return Ok(None);
		}

		// Open the blob.
		let blob = tokio::fs::File::open(path).await?;

		Ok(Some(blob))
	}
}
