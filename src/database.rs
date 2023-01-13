use anyhow::Result;
use std::path::Path;

pub struct Database {
	/// This is the LMDB environment.
	pub env: lmdb::Environment,

	/// This is the artifacts database.
	pub artifacts: lmdb::Database,

	/// This is the packages database.
	pub packages: lmdb::Database,

	/// This is the operations database.
	pub operations: lmdb::Database,

	/// This is the operation children database.
	pub operation_children: lmdb::Database,

	/// This is the operation outputs database.
	pub operation_outputs: lmdb::Database,
}

impl Database {
	pub fn new(path: &Path) -> Result<Database> {
		// Create the env.
		let mut env_builder = lmdb::Environment::new();
		env_builder.set_map_size(1_099_511_627_776);
		env_builder.set_max_dbs(5);
		env_builder.set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR);
		let env = env_builder.open(path)?;

		// Open the artifacts database.
		let artifacts = env.open_db("artifacts".into())?;

		// Open the packages database.
		let packages = env.open_db("packages".into())?;

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
			packages,
			operations,
			operation_children,
			operation_outputs,
		};

		Ok(database)
	}
}
