use super::{Blob, Hash};
use crate::{os, Instance};
use anyhow::{Context, Result};
use tokio::io::AsyncRead;

impl Instance {
	pub async fn get_blob(&self, blob_hash: Hash) -> Result<impl AsyncRead> {
		let blob = self
			.try_get_blob(blob_hash)
			.await?
			.with_context(|| format!(r#"Failed to get blob with hash "{blob_hash}"."#))?;
		Ok(blob)
	}

	pub async fn try_get_blob(&self, blob_hash: Hash) -> Result<Option<impl AsyncRead>> {
		// Get the blob path.
		let path = self.blob_path(blob_hash);

		// Check if the blob exists.
		if !os::fs::exists(&path).await? {
			return Ok(None);
		}

		// Acquire a permit for the blob.
		let permit = self.file_semaphore.clone().acquire_owned().await?;

		// Open the blob file.
		let file = tokio::fs::File::open(path).await?;

		// Create the blob.
		let blob = Blob { file, permit };

		Ok(Some(blob))
	}
}
