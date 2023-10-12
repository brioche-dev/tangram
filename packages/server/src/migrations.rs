use super::Server;
use futures::FutureExt;
use std::path::Path;
use tangram_client::{return_error, Result, WrapErr};

impl Server {
	pub async fn migrate(path: &Path) -> Result<()> {
		let migrations = vec![migration_0000(path).boxed()];

		// Read the version from the version file.
		let version = match tokio::fs::read_to_string(path.join("version")).await {
			Ok(version) => Some(version),
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
			Err(error) => return Err(error.into()),
		};
		let version = if let Some(version) = version {
			Some(
				version
					.trim()
					.parse::<usize>()
					.wrap_err("Failed to read the path format version.")?,
			)
		} else {
			None
		};

		// If this path is from a newer version of Tangram, then return an error.
		if let Some(version) = version {
			if version >= migrations.len() {
				let path = path.display();
				return_error!(
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

async fn migration_0000(path: &Path) -> Result<()> {
	let path = path.to_owned();

	// Create the database.
	let database_path = path.join("database");
	tokio::fs::File::create(&database_path).await?;

	// Open the database.
	let mut env_builder = lmdb::Environment::new();
	env_builder.set_max_dbs(3);
	env_builder.set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR);
	let env = env_builder.open(&database_path)?;

	// Create the objects database.
	env.create_db("objects".into(), lmdb::DatabaseFlags::empty())?;

	// Create the assignments database.
	env.create_db("assignments".into(), lmdb::DatabaseFlags::empty())?;

	// Create the artifacts directory.
	let artifacts_path = path.join("artifacts");
	tokio::fs::create_dir_all(&artifacts_path).await?;

	Ok(())
}
