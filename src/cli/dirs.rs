use std::path::PathBuf;

#[must_use]
pub fn global_config_directory_path() -> Option<PathBuf> {
	if cfg!(target_os = "linux") {
		Some(PathBuf::from("/etc"))
	} else if cfg!(target_os = "macos") {
		Some(PathBuf::from("/Library/Application Support"))
	} else {
		None
	}
}

#[must_use]
pub fn global_data_directory_path() -> Option<PathBuf> {
	if cfg!(any(target_os = "linux", target_os = "macos")) {
		Some(PathBuf::from("/opt"))
	} else {
		None
	}
}

#[must_use]
pub fn user_config_directory_path() -> Option<PathBuf> {
	if cfg!(any(target_os = "linux", target_os = "macos")) {
		Some(home_directory_path()?.join(".config"))
	} else {
		None
	}
}

#[must_use]
pub fn user_data_directory_path() -> Option<PathBuf> {
	if cfg!(any(target_os = "linux", target_os = "macos")) {
		Some(home_directory_path()?.join(".local").join("share"))
	} else {
		None
	}
}

#[must_use]
pub fn home_directory_path() -> Option<PathBuf> {
	if cfg!(any(target_os = "linux", target_os = "macos")) {
		match std::env::var("HOME") {
			Err(_) => None,
			Ok(path) if path.is_empty() => None,
			Ok(path) => Some(PathBuf::from(path)),
		}
	} else {
		None
	}
}
