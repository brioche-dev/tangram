use super::Hash;
use crate::{hash::Writer, Instance};
use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWriteExt};
use tokio_stream::StreamExt;

impl Instance {
	pub async fn add_blob(&self, reader: impl AsyncRead + Unpin) -> Result<Hash> {
		// Get a file permit.
		let permit = self.file_semaphore.acquire().await.unwrap();

		// Create a temp file to read the blob into.
		let temp_path = self.temp_path();
		let mut temp_file = tokio::fs::File::create(&temp_path).await?;

		// Compute the hash of the bytes in the reader and write the bytes to the temp file.
		let mut stream = tokio_util::io::ReaderStream::new(reader);
		let mut hash_writer = Writer::new();
		while let Some(chunk) = stream.next().await {
			let chunk = chunk?;
			hash_writer.update(&chunk);
			temp_file.write_all(&chunk).await?;
		}
		let blob_hash = Hash(hash_writer.finalize());

		// Close the temp file.
		temp_file.sync_all().await?;
		drop(temp_file);

		// Drop the file permit.
		drop(permit);

		// Make the temp file readonly.
		let metadata = tokio::fs::metadata(&temp_path).await?;
		let mut permissions = metadata.permissions();
		permissions.set_readonly(true);
		tokio::fs::set_permissions(&temp_path, permissions).await?;

		// Move the temp file to the blobs path.
		let blob_path = self.blob_path(blob_hash);
		tokio::fs::rename(&temp_path, &blob_path).await?;

		Ok(blob_hash)
	}
}
