use anyhow::Result;
use std::path::Path;

pub async fn migrate(path: &Path) -> Result<()> {
	// Create the database file.
	let path = path.to_owned();
	tokio::fs::File::create(&path.join("database.mdb")).await?;

	// Create the env.
	let database_path = path.join("database.mdb");
	let mut env_builder = lmdb::Environment::new();
	env_builder.set_max_dbs(5);
	env_builder.set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR);
	let env = env_builder.open(&database_path)?;

	// Create the artifacts database.
	env.create_db("artifacts".into(), lmdb::DatabaseFlags::empty())?;

	// Create the packages database.
	env.create_db("packages".into(), lmdb::DatabaseFlags::empty())?;

	// Create the operations database.
	env.create_db("operations".into(), lmdb::DatabaseFlags::empty())?;

	// Create the operation children database.
	let mut flags = lmdb::DatabaseFlags::empty();
	flags.insert(lmdb::DatabaseFlags::DUP_SORT);
	flags.insert(lmdb::DatabaseFlags::DUP_FIXED);
	env.create_db("operation_children".into(), flags)?;

	// Create the operation outputs database.
	env.create_db("operation_outputs".into(), lmdb::DatabaseFlags::empty())?;

	// Create the blobs directory.
	let blobs_path = path.join("blobs");
	tokio::fs::create_dir_all(&blobs_path).await?;

	// Create the checkouts directory.
	let checkouts_path = path.join("checkouts");
	tokio::fs::create_dir_all(&checkouts_path).await?;

	// Create the temps directory.
	let temps_path = path.join("temps");
	tokio::fs::create_dir_all(&temps_path).await?;

	Ok(())
}
