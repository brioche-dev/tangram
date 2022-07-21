use std::path::PathBuf;

#[must_use]
pub fn _global_config_dir() -> Option<PathBuf> {
	if cfg!(target_os = "linux") {
		Some(PathBuf::from("/etc"))
	} else if cfg!(target_os = "macos") {
		Some(PathBuf::from("/Library/Application Support"))
	} else {
		None
	}
}

#[must_use]
pub fn _global_data_dir() -> Option<PathBuf> {
	if cfg!(any(target_os = "linux", target_os = "macos")) {
		Some(PathBuf::from("/opt"))
	} else {
		None
	}
}

#[must_use]
pub fn _user_config_dir() -> Option<PathBuf> {
	if cfg!(any(target_os = "linux", target_os = "macos")) {
		Some(home_dir()?.join(".config"))
	} else {
		None
	}
}

#[must_use]
pub fn _user_data_dir() -> Option<PathBuf> {
	if cfg!(any(target_os = "linux", target_os = "macos")) {
		Some(home_dir()?.join(".local").join("share"))
	} else {
		None
	}
}

#[must_use]
pub fn home_dir() -> Option<PathBuf> {
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
