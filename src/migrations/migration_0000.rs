use crate::error::Result;
use std::path::Path;

pub async fn migrate(path: &Path) -> Result<()> {
	let path = path.to_owned();

	// Create the database.
	let database_path = path.join("database");
	tokio::fs::File::create(&database_path).await?;

	// Open the database.
	let mut env_builder = lmdb::Environment::new();
	env_builder.set_max_dbs(2);
	env_builder.set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR);
	let env = env_builder.open(&database_path)?;

	// Create the evaluations database.
	env.create_db("evaluations".into(), lmdb::DatabaseFlags::empty())?;

	// Create the outputs database.
	env.create_db("outputs".into(), lmdb::DatabaseFlags::empty())?;

	// Create the store.
	let store_path = path.join("store");
	tokio::fs::File::create(&store_path).await?;

	// Open the store.
	let mut env_builder = lmdb::Environment::new();
	env_builder.set_max_dbs(1);
	env_builder.set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR);
	let env = env_builder.open(&store_path)?;

	// Create the blocks database.
	env.create_db("blocks".into(), lmdb::DatabaseFlags::empty())?;

	// Create the artifacts directory.
	let artifacts_path = path.join("artifacts");
	tokio::fs::create_dir_all(&artifacts_path).await?;

	// Create the temps directory.
	let temps_path = path.join("temps");
	tokio::fs::create_dir_all(&temps_path).await?;

	Ok(())
}
