use super::fs;
use crate::error::{error, Error, Result};

#[must_use]
pub fn global_config_directory_path() -> fs::PathBuf {
	if cfg!(target_os = "linux") {
		fs::PathBuf::from("/etc")
	} else if cfg!(target_os = "macos") {
		fs::PathBuf::from("/Library/Application Support")
	} else {
		unimplemented!()
	}
}

#[must_use]
pub fn global_data_directory_path() -> fs::PathBuf {
	if cfg!(any(target_os = "linux", target_os = "macos")) {
		fs::PathBuf::from("/opt")
	} else {
		unimplemented!()
	}
}

pub fn user_config_directory_path() -> Result<fs::PathBuf> {
	if cfg!(any(target_os = "linux", target_os = "macos")) {
		Ok(home_directory_path()?.join(".config"))
	} else {
		unimplemented!()
	}
}

pub fn user_data_directory_path() -> Result<fs::PathBuf> {
	if cfg!(any(target_os = "linux", target_os = "macos")) {
		Ok(home_directory_path()?.join(".local/share"))
	} else {
		unimplemented!()
	}
}

pub fn home_directory_path() -> Result<fs::PathBuf> {
	if cfg!(any(target_os = "linux", target_os = "macos")) {
		match std::env::var("HOME") {
			Err(error) => Err(Error::other(error)),
			Ok(value) if value.is_empty() => {
				Err(error!(r#"The "HOME" environment variable is not set."#))
			},
			Ok(value) => Ok(fs::PathBuf::from(value)),
		}
	} else {
		unimplemented!()
	}
}
