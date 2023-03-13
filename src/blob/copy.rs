use super::Hash;
use crate::{error::Result, util::fs, Instance};
use tokio::io::AsyncWrite;

impl Instance {
	pub async fn copy_blob_to_path(&self, blob_hash: Hash, path: &fs::Path) -> Result<()> {
		// Get the blob path.
		let blob_path = self.blob_path(blob_hash);

		// Acquire a permit and copy the file.
		let permit = self.file_semaphore.acquire().await?;
		tokio::fs::copy(blob_path, path).await?;
		drop(permit);

		Ok(())
	}

	pub async fn copy_blob_to_writer<W>(&self, blob_hash: Hash, writer: &mut W) -> Result<()>
	where
		W: AsyncWrite + Unpin,
	{
		// Get the blob path.
		let path = self.blob_path(blob_hash);

		// Open the file.
		let mut file = tokio::fs::File::open(path).await?;

		// Copy the blob to the writer.
		tokio::io::copy(&mut file, writer).await?;

		Ok(())
	}
}
