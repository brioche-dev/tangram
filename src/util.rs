use crate::Result;
use std::path::Path;

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[must_use]
pub fn errno() -> i32 {
	std::io::Error::last_os_error().raw_os_error().unwrap()
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

pub mod dirs {
	use crate::{error, Error, Result};
	use std::path::PathBuf;

	#[must_use]
	pub fn global_config_directory_path() -> PathBuf {
		#[cfg(target_os = "linux")]
		return PathBuf::from("/etc");
		#[cfg(target_os = "macos")]
		return PathBuf::from("/Library/Application Support");
	}

	#[must_use]
	pub fn global_data_directory_path() -> PathBuf {
		#[cfg(any(target_os = "linux", target_os = "macos"))]
		return PathBuf::from("/opt");
	}

	pub fn user_config_directory_path() -> Result<PathBuf> {
		#[cfg(any(target_os = "linux", target_os = "macos"))]
		return Ok(home_directory_path()?.join(".config"));
	}

	pub fn user_data_directory_path() -> Result<PathBuf> {
		#[cfg(any(target_os = "linux", target_os = "macos"))]
		return Ok(home_directory_path()?.join(".local/share"));
	}

	pub fn home_directory_path() -> Result<PathBuf> {
		#[cfg(any(target_os = "linux", target_os = "macos"))]
		return match std::env::var("HOME") {
			Err(error) => Err(Error::other(error)),
			Ok(value) if value.is_empty() => {
				Err(error!(r#"The "HOME" environment variable is not set."#))
			},
			Ok(value) => Ok(PathBuf::from(value)),
		};
	}
}
