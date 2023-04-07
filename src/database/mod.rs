use crate::{
	artifact,
	error::{Error, Result},
	operation, package,
	util::fs,
	value,
};
use itertools::Itertools;
use lmdb::{Cursor, Transaction};
use std::os::unix::prelude::OsStrExt;

pub struct Database {
	/// The LMDB environment.
	pub env: lmdb::Environment,

	/// The artifacts database maps artifact hashes to artifacts.
	pub artifacts: lmdb::Database,

	/// The artifact trackers database maps paths to artifact trackers.
	pub artifact_trackers: lmdb::Database,

	/// The package instances database maps package instance hashes to package instances.
	pub package_instances: lmdb::Database,

	/// The operations database maps operation hashes to operations.
	pub operations: lmdb::Database,

	/// The operation children database maps operation hashes to multiple operation hashes.
	pub operation_children: lmdb::Database,

	/// The operation outputs database maps operation hashes to values.
	pub operation_outputs: lmdb::Database,
}

impl Database {
	pub fn open(path: &fs::Path) -> Result<Database> {
		// Open the environment.
		let mut env_builder = lmdb::Environment::new();
		env_builder.set_map_size(1_099_511_627_776);
		env_builder.set_max_dbs(6);
		env_builder.set_max_readers(1024);
		env_builder.set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR);
		let env = env_builder.open(path)?;

		// Open the artifacts database.
		let artifacts = env.open_db("artifacts".into())?;

		// Open the artifact trackers database.
		let artifact_trackers = env.open_db("artifact_trackers".into())?;

		// Open the package instances database.
		let package_instances = env.open_db("package_instances".into())?;

		// Open the operations database.
		let operations = env.open_db("operations".into())?;

		// Open the operation children database.
		let operation_children = env.open_db("operation_children".into())?;

		// Open the operation outputs database.
		let operation_outputs = env.open_db("operation_outputs".into())?;

		// Create the database.
		let database = Database {
			env,
			artifacts,
			artifact_trackers,
			package_instances,
			operations,
			operation_children,
			operation_outputs,
		};

		Ok(database)
	}
}

impl Database {
	#[allow(clippy::unused_async)]
	pub async fn add_artifact(&self, hash: artifact::Hash, bytes: &[u8]) -> Result<artifact::Hash> {
		// Begin a write transaction.
		let mut txn = self.env.begin_rw_txn()?;

		// Add the artifact to the database.
		match txn.put(
			self.artifacts,
			&hash.as_slice(),
			&bytes,
			lmdb::WriteFlags::NO_OVERWRITE,
		) {
			Ok(_) | Err(lmdb::Error::KeyExist) => {},
			Err(error) => return Err(error.into()),
		};

		// Commit the transaction.
		txn.commit()?;

		Ok(hash)
	}

	#[allow(clippy::unused_async)]
	pub async fn try_get_artifact(&self, hash: artifact::Hash) -> Result<Option<artifact::Data>> {
		// Begin a read transaction.
		let txn = self.env.begin_ro_txn()?;

		// Get the bytes.
		let bytes = match txn.get(self.artifacts, &hash.as_slice()) {
			Ok(bytes) => bytes,
			Err(lmdb::Error::NotFound) => return Ok(None),
			Err(error) => return Err(error.into()),
		};

		// Deserialize the artifact data.
		let data = artifact::Data::deserialize(bytes)?;

		Ok(Some(data))
	}

	/// Add an artifact tracker.
	pub fn add_artifact_tracker(
		&self,
		path: &fs::Path,
		artifact_tracker: &artifact::Tracker,
	) -> Result<()> {
		// Serialize the artifact tracker.
		let mut bytes = Vec::new();
		artifact_tracker.serialize(&mut bytes).unwrap();

		// Begin a write transaction.
		let mut txn = self.env.begin_rw_txn()?;

		// Add the artifact tracker to the database.
		match txn.put(
			self.artifact_trackers,
			&path.as_os_str().as_bytes(),
			&bytes,
			lmdb::WriteFlags::empty(),
		) {
			Ok(_) | Err(lmdb::Error::KeyExist) => {},
			Err(error) => return Err(error.into()),
		};

		// Commit the transaction.
		txn.commit()?;

		Ok(())
	}

	/// Get an artifact tracker.
	pub fn try_get_artifact_tracker(&self, path: &fs::Path) -> Result<Option<artifact::Tracker>> {
		// Begin a read transaction.
		let txn = self.env.begin_ro_txn()?;

		// Get the bytes.
		let bytes = match txn.get(self.artifact_trackers, &path.as_os_str().as_bytes()) {
			Ok(bytes) => bytes,
			Err(lmdb::Error::NotFound) => return Ok(None),
			Err(error) => return Err(error.into()),
		};

		// Deserialize the artifact tracker.
		let artifact_tracker = artifact::Tracker::deserialize(bytes)?;

		Ok(Some(artifact_tracker))
	}

	#[allow(clippy::unused_async)]
	pub async fn add_package_instance(
		&self,
		hash: package::instance::Hash,
		bytes: &[u8],
	) -> Result<package::instance::Hash> {
		// Begin a write transaction.
		let mut txn = self.env.begin_rw_txn()?;

		// Add the package instance to the database.
		match txn.put(
			self.package_instances,
			&hash.as_slice(),
			&bytes,
			lmdb::WriteFlags::NO_OVERWRITE,
		) {
			Ok(_) | Err(lmdb::Error::KeyExist) => {},
			Err(error) => return Err(error.into()),
		};

		// Commit the transaction.
		txn.commit()?;

		Ok(hash)
	}

	#[allow(clippy::unused_async)]
	pub async fn try_get_package_instance(
		&self,
		hash: package::instance::Hash,
	) -> Result<Option<package::instance::Data>> {
		// Begin a read transaction.
		let txn = self.env.begin_ro_txn()?;

		// Get the data.
		let data = match txn.get(self.package_instances, &hash.as_slice()) {
			Ok(data) => data,
			Err(lmdb::Error::NotFound) => return Ok(None),
			Err(error) => return Err(error.into()),
		};

		// Deserialize the package instance data.
		let data = package::instance::Data::deserialize(data)?;

		Ok(Some(data))
	}

	#[allow(clippy::unused_async)]
	pub async fn add_operation(
		&self,
		hash: operation::Hash,
		bytes: &[u8],
	) -> Result<operation::Hash> {
		// Begin a write transaction.
		let mut txn = self.env.begin_rw_txn()?;

		// Add the operation to the database.
		match txn.put(
			self.operations,
			&hash.as_slice(),
			&bytes,
			lmdb::WriteFlags::NO_OVERWRITE,
		) {
			Ok(_) | Err(lmdb::Error::KeyExist) => {},
			Err(error) => return Err(error.into()),
		};

		// Commit the transaction.
		txn.commit()?;

		Ok(hash)
	}

	/// Try to get an operation from the database.
	#[allow(clippy::unused_async)]
	pub async fn try_get_operation(
		&self,
		hash: operation::Hash,
	) -> Result<Option<operation::Data>> {
		// Begin a read transaction.
		let txn = self.env.begin_ro_txn()?;

		// Get the data.
		let bytes = match txn.get(self.operations, &hash.as_slice()) {
			Ok(bytes) => bytes,
			Err(lmdb::Error::NotFound) => return Ok(None),
			Err(error) => return Err(error.into()),
		};

		// Deserialize the operation data.
		let data = operation::Data::deserialize(bytes)?;

		Ok(Some(data))
	}

	/// Get the output for an operation from the database.
	#[allow(clippy::unused_async)]
	pub async fn get_operation_output(
		&self,
		operation_hash: operation::Hash,
	) -> Result<Option<value::Data>> {
		// Begin a read transaction.
		let txn = self.env.begin_ro_txn()?;

		// Get the data.
		let bytes = match txn.get(self.operation_outputs, &operation_hash.as_slice()) {
			Ok(bytes) => bytes,
			Err(lmdb::Error::NotFound) => return Ok(None),
			Err(error) => return Err(error.into()),
		};

		// Deserialize the value data.
		let data = value::Data::deserialize(bytes)?;

		Ok(Some(data))
	}

	/// Set the output for an operation in the database.
	#[allow(clippy::unused_async)]
	pub async fn set_operation_output(
		&self,
		operation_hash: operation::Hash,
		output_data: &value::Data,
	) -> Result<()> {
		// Begin a write transaction.
		let mut txn = self.env.begin_rw_txn()?;

		// Serialize the output data.
		let mut bytes = Vec::new();
		output_data.serialize(&mut bytes).unwrap();

		// Add the output to the database.
		txn.put(
			self.operation_outputs,
			&operation_hash.as_slice(),
			&bytes,
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		Ok(())
	}

	/// Add a run to the database.
	#[allow(clippy::unused_async)]
	pub async fn add_operation_child(
		&self,
		parent_operation_hash: operation::Hash,
		child_operation_hash: operation::Hash,
	) -> Result<()> {
		// Begin a write transaction.
		let mut txn = self.env.begin_rw_txn()?;

		// Add the operation child.
		txn.put(
			self.operation_children,
			&parent_operation_hash.as_slice(),
			&child_operation_hash.as_slice(),
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		Ok(())
	}

	/// Get the children for an operation.
	#[allow(clippy::unused_async)]
	pub async fn get_operation_children(
		&self,
		operation_hash: operation::Hash,
	) -> Result<Vec<operation::Hash>> {
		// Begin a read transaction.
		let txn = self.env.begin_ro_txn()?;

		// Open a readonly cursor.
		let mut cursor = txn.open_ro_cursor(self.operation_children)?;

		// Get the children.
		let children = cursor
			.iter_dup_of(operation_hash.as_slice())
			.map(|value| {
				let (_, value) = value?;
				let value = buffalo::from_slice(value)?;
				Ok::<_, Error>(value)
			})
			.try_collect()?;

		Ok(children)
	}
}
