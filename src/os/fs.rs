use crate::os;
use anyhow::{bail, Result};
use std::fs::Metadata;

pub async fn exists(path: &os::Path) -> Result<bool> {
	match tokio::fs::metadata(&path).await {
		Ok(_) => Ok(true),
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
		Err(error) => Err(error.into()),
	}
}

pub async fn rmrf(path: &os::Path, metadata: Option<Metadata>) -> Result<()> {
	let metadata = if let Some(metadata) = metadata {
		metadata
	} else {
		tokio::fs::metadata(path).await?
	};

	if metadata.is_dir() {
		tokio::fs::remove_dir_all(path).await?;
	} else if metadata.is_file() {
		tokio::fs::remove_file(path).await?;
	} else if metadata.is_symlink() {
		tokio::fs::remove_file(path).await?;
	} else {
		bail!("The path must point to a directory, file, or symlink.");
	};

	Ok(())
}
