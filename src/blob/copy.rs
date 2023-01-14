use super::BlobHash;
use crate::Cli;
use anyhow::Result;

impl Cli {
	pub async fn copy_blob<W>(&self, blob_hash: BlobHash, mut writer: W) -> Result<()>
	where
		W: std::io::Write + Send + 'static,
	{
		// Get the blob.
		let blob = self.get_blob(blob_hash).await?;

		// Get the blob's std file.
		let mut file = blob.file.into_std().await;

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
