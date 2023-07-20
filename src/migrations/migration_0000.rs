use crate::error::Result;
use std::path::Path;

pub async fn migrate(path: &Path) -> Result<()> {
	let path = path.to_owned();

	// Create the database.
	let database_path = path.join("database");
	let database = rusqlite::Connection::open(&database_path)?;

	// Create the database tables.
	database.execute_batch(
		r#"
			create table blocks (
				id blob primary key,
				bytes blob
			);

			create table outputs (
				id blob primary key,
				value blob not null
			);
		"#,
	)?;

	// Create the artifacts directory.
	let artifacts_path = path.join("artifacts");
	tokio::fs::create_dir_all(&artifacts_path).await?;

	// Create the temps directory.
	let temps_path = path.join("temps");
	tokio::fs::create_dir_all(&temps_path).await?;

	Ok(())
}
