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

		// Acquire a permit and copy the file.
		// Note: We use tokio::fs::copy which calls std::fs::copy under the hood.
		//
		// std::fs::copy has the following behavior:
		//
		// On Linux: calls copy_file_range, a syscall that allows filesystems to perform reflinks/copy-on-write behavior.
		// On MacOS: calls fclonefileat, a syscall that corresponds to shallow clones on APFS.
		//
		// References:
		//     https://doc.rust-lang.org/std/fs/fn.copy.html#errors
		//     https://manpages.ubuntu.com/manpages/impish/man2/copy_file_range.2.html
		//     https://www.manpagez.com/man/2/fclonefileat/
		//
		// Additional notes: when using fs::copy, file watchers may report the source file has changed.
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
