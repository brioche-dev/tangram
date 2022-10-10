use anyhow::{anyhow, Result};
use async_trait::async_trait;
use heed::{flags::Flags, EnvOpenOptions};
use std::path::Path;

use crate::builder::db::{EvaluationsDatabase, ExpressionsDatabase};

pub struct Migration;

#[async_trait]
impl super::Migration for Migration {
	async fn run(&self, path: &Path) -> Result<()> {
		// Create the db file.
		let path = path.to_owned();
		tokio::fs::File::create(&path.join("db.mdb")).await?;

		// Create the env.
		let database_path = path.join("db.mdb");
		let mut env_builder = EnvOpenOptions::new();
		env_builder.max_dbs(2);
		unsafe {
			env_builder.flag(Flags::MdbNoSubDir);
		}
		let env = env_builder
			.open(database_path)
			.map_err(|_| anyhow!("Unable to open the database."))?;

		// Create the expression db.
		let _expressions_db: ExpressionsDatabase = env
			.create_database("expressions".into())
			.map_err(|_| anyhow!("Unable to create the database."))?;

		// Create the evaluations db.
		let _evaluations_db: EvaluationsDatabase = env
			.create_database("evaluations".into())
			.map_err(|_| anyhow!("Unable to create the database."))?;

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
