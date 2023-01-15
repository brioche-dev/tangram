use crate::{system::System, value::Value, Cli};
use anyhow::{bail, Result};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use std::{fs::Metadata, path::Path};

impl Cli {
	pub fn create_target_args(&self, system: Option<System>) -> Result<Vec<Value>> {
		let host = System::host()?;
		let system = system.unwrap_or(host);
		Ok(vec![Value::Map(
			[("target".to_owned(), Value::String(system.to_string()))].into(),
		)])
	}
}

#[must_use]
pub fn normalize(path: &Utf8Path) -> Utf8PathBuf {
	let mut normalized_path = Utf8PathBuf::new();

	for component in path.components() {
		match component {
			Utf8Component::Prefix(prefix) => {
				// Replace the path.
				normalized_path = Utf8PathBuf::from(prefix.to_string());
			},

			Utf8Component::RootDir => {
				// Replace the path.
				normalized_path = Utf8PathBuf::from("/");
			},

			Utf8Component::CurDir => {
				// Skip current dir components.
			},

			Utf8Component::ParentDir => {
				if normalized_path.components().count() == 1
					&& matches!(
						normalized_path.components().next(),
						Some(Utf8Component::Prefix(_) | Utf8Component::RootDir)
					) {
					// If the normalized path has one component which is a prefix or a root dir component, then do nothing.
				} else if normalized_path
					.components()
					.all(|component| matches!(component, Utf8Component::ParentDir))
				{
					// If the normalized path is zero or more parent dir components, then add a parent dir component.
					normalized_path.push("..");
				} else {
					// Otherwise, remove the last component.
					normalized_path.pop();
				}
			},

			Utf8Component::Normal(string) => {
				// Add the component.
				normalized_path.push(string);
			},
		}
	}

	normalized_path
}

pub async fn path_exists(path: &Path) -> Result<bool> {
	match tokio::fs::metadata(&path).await {
		Ok(_) => Ok(true),
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
		Err(error) => Err(error.into()),
	}
}

pub async fn rmrf(path: &Path, metadata: Option<Metadata>) -> Result<()> {
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

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[must_use]
pub fn errno() -> i32 {
	std::io::Error::last_os_error().raw_os_error().unwrap()
}
