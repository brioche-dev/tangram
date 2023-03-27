use super::{reader::Reader, Hash};
use crate::{
	error::{Error, Result, WrapErr},
	Instance,
};
use tokio::io::AsyncRead;

impl Instance {
	pub async fn get_blob(&self, blob_hash: Hash) -> Result<impl AsyncRead> {
		let blob = self
			.try_get_blob(blob_hash)
			.await?
			.wrap_err_with(|| format!(r#"Failed to get the blob with hash "{blob_hash}"."#))?;
		Ok(blob)
	}

	pub async fn try_get_blob(&self, blob_hash: Hash) -> Result<Option<impl AsyncRead>> {
		// Get the blob path.
		let path = self.blob_path(blob_hash);

		// Acquire a permit for the blob.
		let permit = self
			.file_semaphore
			.clone()
			.acquire_owned()
			.await
			.map_err(Error::other)?;

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
