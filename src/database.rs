use crate::{error::Result, util::fs};

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
