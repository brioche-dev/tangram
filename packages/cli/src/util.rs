pub mod dirs {
	use crate::{error, Result};
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
			Err(error) => Err(error.into()),
			Ok(value) if value.is_empty() => {
				Err(error!(r#"The "HOME" environment variable is not set."#))
			},
			Ok(value) => Ok(PathBuf::from(value)),
		};
	}
}
