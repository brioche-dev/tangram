use super::{ModuleIdentifier, Range};
use crate::Cli;
use anyhow::Result;
use std::{path::Path, time::SystemTime};

pub enum TrackedFile {
	Opened(OpenedTrackedFile),
	Unopened(UnopenedTrackedFile),
}

pub struct OpenedTrackedFile {
	pub module_identifier: ModuleIdentifier,
	pub version: i32,
	pub text: String,
}

pub struct UnopenedTrackedFile {
	pub version: i32,
	pub modified: SystemTime,
}

impl Cli {
	pub async fn open_file(&self, path: &Path, version: i32, text: String) {
		// Get the module identifier.
		let Ok(module_identifier) = ModuleIdentifier::for_path(path).await else { return };

		// Create the tracked file.
		let file = TrackedFile::Opened(OpenedTrackedFile {
			module_identifier,
			version,
			text,
		});

		// Add the tracked file.
		self.inner
			.tracked_files
			.write()
			.await
			.insert(path.to_owned(), file);
	}

	pub async fn close_file(&self, path: &Path) {
		self.inner.tracked_files.write().await.remove(path);
	}

	pub async fn change_file(&self, path: &Path, version: i32, range: Option<Range>, text: String) {
		// Lock the files.
		let mut files = self.inner.tracked_files.write().await;

		// Get the file.
		let Some(TrackedFile::Opened(file)) = files.get_mut(path) else { return };

		// Convert the range to bytes.
		let range = if let Some(range) = range {
			let start = byte_index_for_line_and_character_index(
				&file.text,
				range.start.line as usize,
				range.start.character as usize,
			);
			let end = byte_index_for_line_and_character_index(
				&file.text,
				range.end.line as usize,
				range.end.character as usize,
			);
			start..end
		} else {
			0..file.text.len()
		};

		// Replace the text and update the version.
		file.text.replace_range(range, &text);
		file.version = version;
	}

	pub async fn version(&self, module_identifier: &ModuleIdentifier) -> Result<i32> {
		// Get the path for the module identifier, or return version 0 for modules whose contents never change.
		let path = match module_identifier {
			// Path modules change when the file at their path changes.
			ModuleIdentifier::Path {
				package_path,
				module_path,
			} => package_path.join(module_path),

			// Library, and hash modules never change, so we can always return 0.
			ModuleIdentifier::Lib { .. } | ModuleIdentifier::Hash { .. } => {
				return Ok(0);
			},
		};

		let mut files = self.inner.tracked_files.write().await;
		match files.get_mut(&path) {
			// If the file is not tracked, add it as unopened at version 0 and save its modified time.
			None => {
				let metadata = tokio::fs::metadata(&path).await?;
				let modified = metadata.modified()?;
				files.insert(
					path,
					TrackedFile::Unopened(UnopenedTrackedFile {
						version: 0,
						modified,
					}),
				);
				Ok(0)
			},

			// If the tracked file is opened, return its version.
			Some(TrackedFile::Opened(opened_file)) => Ok(opened_file.version),

			// If the tracked file is unopened, update its version if the file's modified time is newer, and return the version.
			Some(TrackedFile::Unopened(unopened_file)) => {
				let metadata = tokio::fs::metadata(&path).await?;
				let modified = metadata.modified()?;
				if modified > unopened_file.modified {
					unopened_file.modified = modified;
					unopened_file.version += 1;
				}
				Ok(unopened_file.version)
			},
		}
	}
}

fn byte_index_for_line_and_character_index(string: &str, line: usize, character: usize) -> usize {
	let mut byte_index = 0;
	let mut line_index = 0;
	let mut character_index = 0;
	for code_point in string.chars() {
		if line_index == line && character_index == character {
			return byte_index;
		}
		byte_index += code_point.len_utf8();
		character_index += 1;
		if code_point == '\n' {
			line_index += 1;
			character_index = 0;
		}
	}
	byte_index
}
