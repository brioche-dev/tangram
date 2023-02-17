use super::Tracker;
use crate::{os, Cli};
use anyhow::Result;
use lmdb::Transaction;
use std::os::unix::prelude::OsStrExt;

impl Cli {
	/// Add an artifact tracker.
	pub fn add_artifact_tracker(&self, path: &os::Path, artifact_tracker: &Tracker) -> Result<()> {
		// Serialize the artifact tracker.
		let value = artifact_tracker.serialize_to_vec();

		// Begin a write transaction.
		let mut txn = self.database.env.begin_rw_txn()?;

		// Add the artifact tracker to the database.
		match txn.put(
			self.database.artifact_trackers,
			&path.as_os_str().as_bytes(),
			&value,
			lmdb::WriteFlags::empty(),
		) {
			Ok(_) | Err(lmdb::Error::KeyExist) => Ok(()),
			Err(error) => Err(error),
		}?;

		// Commit the transaction.
		txn.commit()?;

		Ok(())
	}
}
