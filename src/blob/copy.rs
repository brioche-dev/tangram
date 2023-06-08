use super::Blob;
use crate::{error::return_error, error::Result, instance::Instance};
use std::path::Path;

impl Blob {
	pub async fn copy_to_path(&self, tg: &Instance, path: &Path) -> Result<()> {
		let blob_path = tg.blob_path(self.hash);
		if tokio::fs::try_exists(&blob_path).await? {
			// Attempt to copy the blob locally.
			tokio::fs::copy(&blob_path, path).await?;
		} else {
			// Otherwise, attempt to copy the blob from the API.
			let reader = tg.api_client().try_get_blob(self.hash).await.ok().flatten();
			let Some(mut reader) = reader else {
				return_error!("Failed to find the blob.");
			};
			let mut file = tokio::fs::File::create(path).await?;
			tokio::io::copy(&mut reader, &mut file).await?;
		}
		Ok(())
	}
}
