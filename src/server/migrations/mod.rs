use anyhow::{bail, Result};
use async_trait::async_trait;
use sqlx::prelude::*;
use std::collections::BTreeMap;

mod migration_2022_01_01_000000;

#[async_trait]
trait Migration: Send + Sync {
	async fn run(&self, txn: &mut sqlx::Transaction<sqlx::Sqlite>) -> Result<()>;
}

type Migrations = BTreeMap<&'static str, Box<dyn Migration>>;

fn migrations() -> Migrations {
	let mut migrations: Migrations = BTreeMap::new();
	migrations.insert(
		"2022_01_01_000000",
		Box::new(migration_2022_01_01_000000::Migration),
	);
	migrations
}

/// Determine if any migrations have been run on the database yet.
pub async fn empty(db: &sqlx::SqlitePool) -> Result<bool> {
	create_migrations_table_if_necessary(db).await?;
	let empty = sqlx::query("select count(*) = 0 from _migrations")
		.fetch_one(db)
		.await?
		.get(0);
	Ok(empty)
}

/// Verify that the database has run all migrations.
pub async fn verify(db: &sqlx::SqlitePool) -> Result<()> {
	let migrations = migrations();
	create_migrations_table_if_necessary(db).await?;
	let migration_rows = sqlx::query("select name from _migrations order by name")
		.fetch_all(db)
		.await?;
	let migrations_consistent = std::iter::zip(migration_rows.iter(), migrations.keys()).all(
		|(migration_row, migration_name)| migration_row.get::<String, usize>(0) == *migration_name,
	);
	if !migrations_consistent {
		bail!(
			"There was a mismatch between the migrations your database has run and the migrations this version of tangram expects. This should not happen unless you are hacking on tangram. Please contact us at help@tangram.dev."
		);
	}
	if migration_rows.len() > migrations.len() {
		bail!(
			"Your database has run migrations from a newer version of tangram. Please update to the latest version of tangram."
		);
	}
	if migration_rows.len() < migrations.len() {
		bail!("Please run `tangram migrate` to update your database to the latest schema.");
	}
	Ok(())
}

/// Run all outstanding migrations.
pub async fn run(db: &sqlx::SqlitePool) -> Result<()> {
	let migrations = migrations();
	create_migrations_table_if_necessary(db).await?;
	let mut txn = db.begin().await?;
	let migration_rows = sqlx::query("select name from _migrations order by name")
		.fetch_all(&mut txn)
		.await?;
	let migrations_consistent = std::iter::zip(migration_rows.iter(), migrations.keys()).all(
		|(migration_row, migration_name)| migration_row.get::<String, usize>(0) == *migration_name,
	);
	if !migrations_consistent {
		bail!("Database migration consistency error. Please contact us at help@tangram.dev.");
	}
	if migration_rows.len() > migrations.len() {
		bail!("Your database has run migrations from a newer version of tangram.");
	}
	// Apply each outstanding migration in a transaction.
	for (migration_name, migration) in migrations.into_iter().skip(migration_rows.len()) {
		let mut txn = txn.begin().await?;
		migration.run(&mut txn).await?;
		sqlx::query(
			"
				insert into _migrations (name) values ($1)
			",
		)
		.bind(migration_name)
		.execute(&mut txn)
		.await?;
		txn.commit().await?;
	}
	txn.commit().await?;
	Ok(())
}

/// Create the _migrations table if necessary.
async fn create_migrations_table_if_necessary(db: &sqlx::SqlitePool) -> Result<()> {
	sqlx::query(
		"
			create table if not exists _migrations (
				name text primary key
			)
		",
	)
	.execute(db)
	.await?;
	Ok(())
}
