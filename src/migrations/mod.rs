use crate::{os, Instance};
use anyhow::{bail, Context, Result};
use futures::FutureExt;

mod migration_0000;

impl Instance {
	pub async fn migrate(path: &os::Path) -> Result<()> {
		let migrations = vec![migration_0000::migrate(path).boxed()];

		// Get the version from the version file.
		let version_file_path = path.join("version");
		let version_file_exists = os::fs::exists(&version_file_path).await?;
		let version: Option<usize> = if version_file_exists {
			let version = tokio::fs::read_to_string(path.join("version"))
				.await?
				.trim()
				.parse()
				.context("Failed to read the path format version.")?;
			Some(version)
		} else {
			None
		};

		// If this path is from a newer version of Tangram, then return an error.
		if let Some(version) = version {
			if version >= migrations.len() {
				let path = path.display();
				bail!(
					r#"The path "{path}" has run migrations from a newer version of Tangram. Please run `tg upgrade` to upgrade to the latest version of Tangram."#
				);
			}
		}

		// Run all migrations and update the version file.
		let previously_run_migrations_count = version.map_or(0, |version| version + 1);
		let migrations = migrations
			.into_iter()
			.enumerate()
			.skip(previously_run_migrations_count);
		for (version, migration) in migrations {
			// Run the migration.
			migration.await?;

			// Update the version.
			tokio::fs::write(path.join("version"), version.to_string()).await?;
		}

		Ok(())
	}
}
