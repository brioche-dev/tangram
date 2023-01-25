use super::BlobHash;
use crate::{util::path_exists, Cli};
use anyhow::{bail, Result};
use std::path::Path;
use tokio::io::AsyncWrite;

/// Helper trait to control the behavior of copying a file given its path.
#[async_trait::async_trait]
pub trait CopyFromPath {
	/// Copy the data from a file at [path].
	async fn copy_from(&mut self, path: &std::path::Path) -> Result<()>;
}

// For Stdout, we open the file and use std::io::copy.
#[async_trait::async_trait]
impl CopyFromPath for std::io::Stdout {
	async fn copy_from(&mut self, path: &std::path::Path) -> Result<()> {
		let file = tokio::fs::File::open(path).await?;
		let mut file = file.into_std().await;
		std::io::copy(&mut file, self)?;
		Ok(())
	}
}

// When copying a file to another path, we want to make sure to use the most efficient copying mechanism available.
// On Linux we can use the same API as Stdout (std::io::copy_file, which falls back to the sendfile, splice, and
// copy_file_range APIs) however on MacOS we need to use std::fs::copy which allows for shallow clones on APFS.
#[async_trait::async_trait]
#[cfg(any(target_os = "macos", target_os = "linux"))]
impl CopyFromPath for std::path::PathBuf {
	async fn copy_from(&mut self, path: &std::path::Path) -> Result<()> {
		// On MacOS use std::fs::copy.
		#[cfg(target_os = "macos")]
		{
			std::fs::copy(path, self)?;
		}
		// On Linux, use std::io::copy.
		#[cfg(target_os = "linux")]
		{
			let file = tokio::fs::File::open(path).await?;
			let mut file = file.into_std().await;
			std::io::copy(&mut file, self)?;
			Ok(())
		}
		Ok(())
	}
}

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
