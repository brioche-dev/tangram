use crate::{util::path_exists, Cli};
use anyhow::{bail, Context, Result};
use futures::FutureExt;
use std::path::Path;

mod migration_0000;

impl Cli {
	pub async fn migrate(path: &Path) -> Result<()> {
		let migrations = vec![migration_0000::migrate(path).boxed()];

		// Get the path format version.
		let version_file_path = path.join("version");
		let version_file_exists = path_exists(&version_file_path).await?;
		let path_format_version: Option<usize> = if version_file_exists {
			let version = tokio::fs::read_to_string(path.join("version"))
				.await?
				.trim()
				.parse()
				.context("Failed to read the path format version.")?;
			Some(version)
		} else {
			None
		};

		// If this path is from a newer version of tangram, we cannot migrate it.
		if let Some(path_format_version) = path_format_version {
			if path_format_version >= migrations.len() {
				let path = path.display();
				bail!(
					r#"The path "{path}" has run migrations from a newer version of tangram. Please run `tg upgrade` to upgrade to the latest version of tangram."#
				);
			}
		}

		// Run all migrations to update the path to the latest path format version.
		let previously_run_migrations_count = path_format_version.map_or(0, |version| version + 1);
		let migrations = migrations
			.into_iter()
			.enumerate()
			.skip(previously_run_migrations_count);
		for (path_format_version, migration) in migrations {
			// Run the migration.
			migration.await?;

			// Update the path format version.
			tokio::fs::write(path.join("version"), path_format_version.to_string()).await?;
		}

		Ok(())
	}
}
