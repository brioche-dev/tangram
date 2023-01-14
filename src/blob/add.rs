use super::BlobHash;
use crate::{hash::Hasher, Cli};
use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWriteExt};
use tokio_stream::StreamExt;

impl Cli {
	pub async fn add_blob(&self, reader: impl AsyncRead + Unpin) -> Result<BlobHash> {
		// Get a file system permit.
		let permit = self.inner.file_system_semaphore.acquire().await.unwrap();

		// Create a temp file to read the blob into.
		let temp_path = self.temp_path();
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
}
