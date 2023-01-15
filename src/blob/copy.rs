use super::BlobHash;
use crate::{util::path_exists, Cli};
use anyhow::{bail, Result};

impl Cli {
	pub async fn copy_blob<W>(&self, blob_hash: BlobHash, mut writer: W) -> Result<()>
	where
		W: std::io::Write + Send + 'static,
	{
		// Get the blob path.
		let path = self.blob_path(blob_hash);

		// Check if the blob exists.
		if !path_exists(&path).await? {
			bail!(r#"Failed to get blob with hash "{blob_hash}"."#);
		}

		// Open the file.
		let file = tokio::fs::File::open(path).await?;

		// Get the std file.
		let mut file = file.into_std().await;

		// Copy the blob to the writer. Use std::io::copy to ensure reflinking is used where possible.
		tokio::task::spawn_blocking(move || {
			std::io::copy(&mut file, &mut writer)?;
			Ok::<_, anyhow::Error>(())
		})
		.await
		.unwrap()?;

		Ok(())
	}
}
