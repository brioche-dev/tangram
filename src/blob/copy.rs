use super::BlobHash;
use crate::{util::path_exists, Cli};
use anyhow::{bail, Result};
use std::path::Path;
use tokio::io::AsyncWrite;

impl Cli {
	pub async fn copy_blob_to_path(
		&self,
		blob_hash: BlobHash,
		path: impl AsRef<Path>,
	) -> Result<()> {
		// Get the blob path.
		let blob_path = self.blob_path(blob_hash);

		// Check if the blob exists.
		if !path_exists(&blob_path).await? {
			bail!(r#"Failed to get blob with hash "{blob_hash}"."#);
		}

		// Acqwuire a permit and copy the file.
		// TODO: should use tokio::spawn?
		let permit = self.inner.file_semaphore.acquire().await?;
		tokio::fs::copy(blob_path, path).await?;
		drop(permit);
	
		Ok(())
	}

	pub async fn copy_blob_to_writer<W>(&self, blob_hash: BlobHash, writer: &mut W) -> Result<()>
	where
		W: AsyncWrite + Unpin,
	{
		// Get the blob path.
		let path = self.blob_path(blob_hash);

		// Check if the blob exists.
		if !path_exists(&path).await? {
			bail!(r#"Failed to get blob with hash "{blob_hash}"."#);
		}

		// Open the file.
		let mut file = tokio::fs::File::open(path).await?;

		// Copy the blob to the writer.
		tokio::io::copy(&mut file, writer).await?;

		Ok(())
	}
}
