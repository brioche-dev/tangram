use bytes::Bytes;
use lmdb::Transaction;
use std::{os::unix::prelude::OsStrExt, path::Path};
use tangram_client as tg;
use tangram_error::{Result, Wrap, WrapErr};

#[derive(Debug)]
pub struct Database {
	pub env: lmdb::Environment,
	pub objects: lmdb::Database,
	pub assignments: lmdb::Database,
	pub trackers: lmdb::Database,
}

impl Database {
	pub fn open(path: &Path) -> Result<Self> {
		let mut env_builder = lmdb::Environment::new();
		env_builder.set_map_size(1_099_511_627_776);
		env_builder.set_max_dbs(3);
		env_builder.set_max_readers(1024);
		env_builder.set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR);
		let env = env_builder
			.open(path)
			.wrap_err("Failed to open the database.")?;
		let objects = env
			.open_db(Some("objects"))
			.wrap_err("Failed to open the objects database.")?;
		let assignments = env
			.open_db(Some("assignments"))
			.wrap_err("Failed to open the assignments database.")?;
		let trackers = env
			.open_db(Some("trackers"))
			.wrap_err("Failed to open the trackers datatabse.")?;
		let database = Database {
			env,
			objects,
			assignments,
			trackers,
		};
		Ok(database)
	}

	pub fn get_object_exists(&self, id: &tg::object::Id) -> Result<bool> {
		let txn = self
			.env
			.begin_ro_txn()
			.wrap_err("Failed to create the transaction.")?;
		let exists = match txn.get(self.objects, &id.to_bytes()) {
			Ok(_) => true,
			Err(lmdb::Error::NotFound) => false,
			Err(error) => return Err(error.wrap("Failed to get the object.")),
		};
		Ok(exists)
	}

	pub fn try_get_object(&self, id: &tg::object::Id) -> Result<Option<Bytes>> {
		let txn = self
			.env
			.begin_ro_txn()
			.wrap_err("Failed to create the transaction.")?;
		let bytes = match txn.get(self.objects, &id.to_bytes()) {
			Ok(bytes) => Bytes::copy_from_slice(bytes),
			Err(lmdb::Error::NotFound) => return Ok(None),
			Err(error) => return Err(error.wrap("Failed to get the object.")),
		};
		Ok(Some(bytes))
	}

	pub fn put_object(&self, id: &tg::object::Id, bytes: &Bytes) -> Result<()> {
		// Create a write transaction.
		let mut txn = self
			.env
			.begin_rw_txn()
			.wrap_err("Failed to create the transaction.")?;

		// Add the object to the database.
		txn.put(
			self.objects,
			&id.to_bytes(),
			&bytes,
			lmdb::WriteFlags::empty(),
		)
		.wrap_err("Failed to put the object.")?;

		// Commit the transaction.
		txn.commit().wrap_err("Failed to commit the transaction.")?;

		Ok(())
	}

	pub fn try_get_build_for_target(
		&self,
		target_id: &tg::target::Id,
	) -> Result<Option<tg::build::Id>> {
		let txn = self
			.env
			.begin_ro_txn()
			.wrap_err("Failed to create the transaction.")?;
		let bytes = match txn.get(self.assignments, &target_id.to_bytes()) {
			Ok(bytes) => bytes,
			Err(lmdb::Error::NotFound) => return Ok(None),
			Err(error) => return Err(error.wrap("Failed to get the build.")),
		};
		let build_id = bytes.try_into().wrap_err("Invalid ID.")?;
		Ok(Some(build_id))
	}

	pub fn set_build_for_target(
		&self,
		target_id: &tg::target::Id,
		build_id: &tg::build::Id,
	) -> Result<()> {
		// Create a write transaction.
		let mut txn = self
			.env
			.begin_rw_txn()
			.wrap_err("Failed to create the transaction.")?;

		// Add the assignment to the database.
		txn.put(
			self.assignments,
			&target_id.to_bytes(),
			&build_id.to_bytes(),
			lmdb::WriteFlags::empty(),
		)
		.wrap_err("Failed to put the assignment.")?;

		// Commit the transaction.
		txn.commit().wrap_err("Failed to commit the transaction.")?;

		Ok(())
	}

	pub fn try_get_tracker(&self, path: &Path) -> Result<Option<tg::Tracker>> {
		let txn = self
			.env
			.begin_ro_txn()
			.wrap_err("Failed to begin the transaction.")?;
		let data = match txn.get(self.trackers, &path.as_os_str().as_bytes()) {
			Ok(data) => data,
			Err(lmdb::Error::NotFound) => return Ok(None),
			Err(error) => return Err(error.wrap("Failed to get the tracker.")),
		};
		let tracker =
			serde_json::from_slice(data).wrap_err("Failed to deserialize the tracker.")?;
		Ok(Some(tracker))
	}

	pub fn set_tracker(&self, path: &Path, tracker: &tg::Tracker) -> Result<()> {
		// Serialize the tracker.
		let tracker = serde_json::to_vec(&tracker).wrap_err("Failed to serialize the tracker.")?;

		// Create a write transaction.
		let mut txn = self
			.env
			.begin_rw_txn()
			.wrap_err("Failed to create the transaction.")?;

		// Add the tracker to the database.
		txn.put(
			self.trackers,
			&path.as_os_str().as_bytes(),
			&tracker,
			lmdb::WriteFlags::empty(),
		)
		.wrap_err("Failed to put the assignment.")?;

		// Commit the transaction.
		txn.commit().wrap_err("Failed to commit the transaction.")?;

		Ok(())
	}
}
