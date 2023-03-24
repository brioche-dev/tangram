use super::Identifier;
use crate::{error::Result, module, Instance};
use std::time::SystemTime;

/// Tracks a module's version as it changes.
pub struct Tracker {
	pub version: i32,
	pub modified: SystemTime,
}

impl Instance {
	/// Add a module tracker.
	pub async fn add_module_tracker(
		&self,
		module_identifier: &Identifier,
		module_tracker: Tracker,
	) {
		self.module_trackers
			.write()
			.await
			.insert(module_identifier.clone(), module_tracker);
	}

	/// Remove a module tracker.
	pub async fn remove_module_tracker(&self, module_identifier: &Identifier) {
		self.module_trackers.write().await.remove(module_identifier);
	}
}

impl Instance {
	/// Get a module's version.
	pub async fn get_module_version(&self, module_identifier: &module::Identifier) -> Result<i32> {
		match &module_identifier.source {
			// A module whose source is a path changes when the file system object at the path changes.
			module::identifier::Source::Path(package_path) => {
				let path = package_path.join(module_identifier.path.to_string());
				let mut module_trackers = self.module_trackers.write().await;
				let version = match module_trackers.get_mut(module_identifier) {
					// If there is no module tracker, then add one at version 0 and save its modified time.
					None => {
						let metadata = tokio::fs::metadata(&path).await?;
						let modified = metadata.modified()?;
						let version = 0;
						let tracker = Tracker { version, modified };
						module_trackers.insert(module_identifier.clone(), tracker);
						version
					},

					// If there is a module tracker that is not open, then increment its version if the file's modified time is newer, and return the version.
					Some(tracker) => {
						let metadata = tokio::fs::metadata(&path).await?;
						let modified = metadata.modified()?;
						if modified > tracker.modified {
							tracker.modified = modified;
							tracker.version += 1;
						}
						tracker.version
					},
				};

				Ok(version)
			},

			// All other modules never change, so we can always return 0.
			_ => Ok(0),
		}
	}
}
