use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

pub struct Migration;

#[async_trait]
impl super::Migration for Migration {
	async fn run(&self, path: &Path) -> Result<()> {
		// Create the db file.
		let path = path.to_owned();
		tokio::fs::File::create(&path.join("db.mdb")).await?;

		// Create the env.
		let database_path = path.join("db.mdb");
		let mut env_builder = lmdb::Environment::new();
		env_builder.set_max_dbs(2);
		env_builder.set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR);
		let env = env_builder.open(&database_path)?;

		// Create the expression db.
		let _expressions_db = env.create_db("expressions".into(), lmdb::DatabaseFlags::empty())?;

		// Create the evaluations db.
		let mut flags = lmdb::DatabaseFlags::empty();
		flags.insert(lmdb::DatabaseFlags::DUP_SORT);
		flags.insert(lmdb::DatabaseFlags::DUP_FIXED);
		let _evaluations_db = env.create_db("evaluations".into(), flags)?;

		// Create the blobs directory.
		let blobs_path = path.join("blobs");
		tokio::fs::create_dir_all(&blobs_path).await?;

		// Create the artifacts directory.
		let artifacts_path = path.join("artifacts");
		tokio::fs::create_dir_all(&artifacts_path).await?;

		// Create the temps directory.
		let temps_path = path.join("temps");
		tokio::fs::create_dir_all(&temps_path).await?;

		Ok(())
	}
}
