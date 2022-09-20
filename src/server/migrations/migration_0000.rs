use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

const SQL: &str = r#"
	create table expressions (
		hash blob primary key,
		data blob not null,
		output_hash blob
	);

	create table evaluations (
		parent_hash blob not null,
		child_hash blob not null,
		primary key (parent_hash, child_hash)
	);

	create table roots (
		hash blob primary key,
		fragment bool not null
	);

	create table packages (
		name text primary key
	);

	create table package_versions (
		name text not null,
		version text not null,
		hash blob not null,
		primary key (name, version)
	);

	create table autoshells (
		path text not null,
		primary key (path)
	);
"#;

pub struct Migration;

#[async_trait]
impl super::Migration for Migration {
	async fn run(&self, path: &Path) -> Result<()> {
		// Create the database and create the initial set of tables.
		tokio::task::block_in_place(move || -> Result<_> {
			let database_connection = rusqlite::Connection::open(path.join("db.sqlite3"))?;
			database_connection.execute_batch(SQL)?;
			Ok(())
		})?;

		// Create the blobs directory.
		let blobs_path = path.join("blobs");
		tokio::fs::create_dir_all(&blobs_path).await?;

		// Create the fragments directory.
		let fragments_path = path.join("fragments");
		tokio::fs::create_dir_all(&fragments_path).await?;

		// Create the temps directory.
		let temps_path = path.join("temps");
		tokio::fs::create_dir_all(&temps_path).await?;

		Ok(())
	}
}
