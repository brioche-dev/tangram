use super::{Package, PackageHash};
use crate::{artifact::ArtifactHash, lockfile::Lockfile, Cli};
use anyhow::{Context, Result};
use std::path::Path;
use tokio::io::AsyncReadExt;

impl Cli {
	/// Check in a package at the specified path.
	pub async fn checkin_package(&self, path: &Path, locked: bool) -> Result<PackageHash> {
		// Check in the path.
		let package_source_hash = self
			.checkin(path)
			.await
			.context("Failed to check in the package.")?;

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

	/// Check in a package with the specified source artifact hash.
	pub async fn checkin_package_from_artifact_hash(
		&self,
		package_source_hash: ArtifactHash,
	) -> Result<PackageHash> {
		// Get the path.
		let mut path = self.checkouts_path();
		path.push(package_source_hash.to_string());

		// Get the package artifact.
		let package_artifact = self.get_artifact_local(package_source_hash)?;

		// Get the package directory.
		let artifact_directory = package_artifact.as_directory().unwrap();

		// Get the lockfile artifact.
		let lockfile_artifact_hash = artifact_directory.entries.get("tangram.lock").unwrap();
		let lockfile_artifact = self.get_artifact_local(*lockfile_artifact_hash)?;

		// Get the lockfile blob.
		let mut lockfile_blob = self
			.get_blob(lockfile_artifact.as_file().unwrap().blob)
			.await?;

		// Read the lockfile
		let mut lockfile = Vec::new();
		lockfile_blob
			.read_to_end(&mut lockfile)
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
