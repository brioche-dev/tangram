use super::{reader::Reader, Hash};
use crate::{
	error::{Context, Result},
	Instance,
};
use tokio::io::AsyncRead;

impl Instance {
	pub async fn get_blob(&self, blob_hash: Hash) -> Result<impl AsyncRead> {
		let blob = self
			.try_get_blob(blob_hash)
			.await?
			.with_context(|| format!(r#"Failed to get the blob with hash "{blob_hash}"."#))?;
		Ok(blob)
	}

	pub async fn try_get_blob(&self, blob_hash: Hash) -> Result<Option<impl AsyncRead>> {
		// Get the blob path.
		let path = self.blobs_path().join(blob_hash.to_string());

		// Acquire a permit for the blob.
		let permit = self.file_semaphore.clone().acquire_owned().await?;

		// Open the blob file.
		let file = match tokio::fs::File::open(path).await {
			Ok(file) => file,
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
			Err(error) => return Err(error.into()),
		};

		// Create the blob reader.
		let blob = Reader { file, permit };

		Ok(Some(blob))
	}
}
