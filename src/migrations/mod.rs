use crate::{builder::Builder, util::path_exists};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use std::path::Path;

mod migration_0000;

#[async_trait]
trait Migration: Send + Sync {
	async fn run(&self, path: &Path) -> Result<()>;
}

impl Builder {
	pub(super) async fn migrate(path: &Path) -> Result<()> {
		let migrations = vec![Box::new(migration_0000::Migration) as Box<dyn Migration>];

		// Retrieve the path format version.
		let version_file_path = path.join("version");
		let version_file_exists = path_exists(&version_file_path).await?;
		let path_format_version: usize = if version_file_exists {
			tokio::fs::read_to_string(path.join("version"))
				.await?
				.trim()
				.parse()
				.context("Failed to read the path format version.")?
		} else {
			0
		};

		// If this path is from a newer version of tangram, we cannot migrate it.
		if path_format_version > migrations.len() {
			bail!(
				r#"The server path "{}" has run migrations from a newer version of tangram. Please update to the latest version of tangram."#,
				path.display(),
			);
		}

		// Run all migrations to update the path to the latest path format version.
		for (path_format_version, migration) in
			migrations.into_iter().enumerate().skip(path_format_version)
		{
			// Run the migration.
			migration.run(path).await?;

			// Update the path format version.
			tokio::fs::write(path.join("version"), (path_format_version + 1).to_string()).await?;
		}

		Ok(())
	}
}
