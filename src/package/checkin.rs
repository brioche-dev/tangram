use super::{Package, PackageHash};
use crate::{lockfile::Lockfile, Cli};
use anyhow::{Context, Result};
use std::path::Path;

impl Cli {
	/// Check in a package at the specified path.
	pub async fn checkin_package(&self, path: &Path, locked: bool) -> Result<PackageHash> {
		// Generate the lockfile if necessary.
		if !locked {
			self.generate_lockfile(path, locked)
				.await
				.with_context(|| {
					format!(
						r#"Failed to generate the lockfile for the package at path "{}"."#,
						path.display(),
					)
				})?;
		}

		// Check in the path.
		let package_source_hash = self
			.checkin(path)
			.await
			.context("Failed to check in the package.")?;

		// Read the lockfile.
		let lockfile_path = path.join("tangram.lock");
		let lockfile = tokio::fs::read(&lockfile_path)
			.await
			.context("Failed to read the lockfile.")?;
		let lockfile: Lockfile =
			serde_json::from_slice(&lockfile).context("Failed to deserialize the lockfile.")?;

		// Create the package.
		let dependencies = lockfile
			.as_v1()
			.context("Expected V1 Lockfile.")?
			.dependencies
			.iter()
			.map(|(name, entry)| (name.clone(), entry.hash))
			.collect();
		let package = Package {
			source: package_source_hash,
			dependencies,
		};

		// Add the package to the database.
		let package_hash = self.add_package(&package)?;

		Ok(package_hash)
	}
}
