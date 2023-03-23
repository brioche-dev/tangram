use super::Tracker;
use crate::{error::Result, util::fs, Instance};
use lmdb::Transaction;
use std::os::unix::prelude::OsStrExt;

impl Instance {
	/// Get an artifact tracker.
	pub fn get_artifact_tracker(&self, path: &fs::Path) -> Result<Option<Tracker>> {
		// Begin a read transaction.
		let txn = self.database.env.begin_ro_txn()?;

		// Get the artifact tracker.
		match txn.get(
			self.database.artifact_trackers,
			&path.as_os_str().as_bytes(),
		) {
			Ok(value) => {
				let value = Tracker::deserialize(value)?;
				Ok(Some(value))
			},
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => Err(error.into()),
		}
	}
}
