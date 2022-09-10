use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

const SQL: &str = r#"
	create table objects (
		hash blob primary key,
		data blob not null
	);

	create table artifacts (
		object_hash blob primary key,
		foreign key (object_hash) references objects (hash)
	);

	create table evaluations (
		input_hash blob primary key,
		input blob not null,
		output_hash blob not null,
		output blob not null
	);

	create table subexpressions (
		input_hash blob primary key,
		output_hash blob not null,
	);

	create table packages (
		name text primary key
	);

	create table package_versions (
		name text,
		version text,
		artifact_hash blob,
		foreign key (artifact_hash) references artifacts (object_hash),
		primary key (name, version)
	);

	create table roots (
		expression_hash blob primary key,
		fragment bool,
		foreign key (expression_hash) references expressions (hash)
	);
"#;

pub struct Migration;

#[async_trait]
impl super::Migration for Migration {
	async fn run(&self, path: &Path) -> Result<()> {
		// Create the database and create the objects, artifacts, and expressions tables.
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
