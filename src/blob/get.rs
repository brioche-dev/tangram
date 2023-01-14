use super::{Blob, BlobHash};
use crate::{util::path_exists, Cli};
use anyhow::{Context, Result};

impl Cli {
	pub async fn get_blob(&self, blob_hash: BlobHash) -> Result<Blob> {
		let blob = self
			.try_get_blob(blob_hash)
			.await?
			.with_context(|| format!(r#"Failed to get blob with hash "{blob_hash}"."#))?;
		Ok(blob)
	}

	pub async fn try_get_blob(&self, blob_hash: BlobHash) -> Result<Option<Blob>> {
		// Get the blob path.
		let path = self.blob_path(blob_hash);

		// Check if the blob exists.
		if !path_exists(&path).await? {
			return Ok(None);
		}

		// Open the blob.
		let permit = self
			.inner
			.file_system_semaphore
			.clone()
			.acquire_owned()
			.await?;
		let file = tokio::fs::File::open(path).await?;
		let blob = Blob::new(permit, file);

		Ok(Some(blob))
	}
}
