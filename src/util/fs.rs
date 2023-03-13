use crate::error::Result;
pub use std::path::{Component, Path, PathBuf};

pub async fn exists(path: &Path) -> Result<bool> {
	match tokio::fs::metadata(&path).await {
		Ok(_) => Ok(true),
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
		Err(error) => Err(error.into()),
	}
}

pub async fn rmrf(path: &Path) -> Result<()> {
	// Get the metadata for the path.
	let metadata = match tokio::fs::metadata(path).await {
		Ok(metadata) => Ok(metadata),

		// If there is no file system object at the path, then return.
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),

		Err(error) => Err(error),
	}?;

	if metadata.is_dir() {
		tokio::fs::remove_dir_all(path).await?;
	} else {
		tokio::fs::remove_file(path).await?;
	};

	Ok(())
}
