#![allow(clippy::module_name_repetitions)]

pub use std::path::{Component, Path, PathBuf};

#[must_use]
pub fn normalize(path: &Path) -> PathBuf {
	let mut normalized_path = PathBuf::new();

	for component in path.components() {
		match component {
			Component::Prefix(prefix) => {
				// Replace the path.
				normalized_path = PathBuf::from(prefix.as_os_str().to_owned());
			},

			Component::RootDir => {
				// Replace the path.
				normalized_path = PathBuf::from("/");
			},

			Component::CurDir => {
				// Skip current dir components.
			},

			Component::ParentDir => {
				if normalized_path.components().count() == 1
					&& matches!(
						normalized_path.components().next(),
						Some(Component::Prefix(_) | Component::RootDir)
					) {
					// If the normalized path has one component which is a prefix or a root dir component, then do nothing.
				} else if normalized_path
					.components()
					.all(|component| matches!(component, Component::ParentDir))
				{
					// If the normalized path is zero or more parent dir components, then add a parent dir component.
					normalized_path.push("..");
				} else {
					// Otherwise, remove the last component.
					normalized_path.pop();
				}
			},

			Component::Normal(string) => {
				// Add the component.
				normalized_path.push(string);
			},
		}
	}

	normalized_path
}
