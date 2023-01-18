use super::{Package, PackageHash};
use crate::{artifact::ArtifactHash, manifest::Manifest, Cli};
use anyhow::{bail, Context, Result};
use camino::Utf8PathBuf;
use lmdb::Transaction;
use tokio::io::AsyncReadExt;

impl Cli {
	pub fn package_exists_local(&self, package_hash: PackageHash) -> Result<bool> {
		// Begin a read transaction.
		let txn = self.inner.database.env.begin_ro_txn()?;

		let exists = match txn.get(self.inner.database.packages, &package_hash.as_slice()) {
			Ok(_) => Ok::<_, anyhow::Error>(true),
			Err(lmdb::Error::NotFound) => Ok(false),
			Err(error) => Err(error.into()),
		}?;

		Ok(exists)
	}

	/// Try to get a package from the database with the given transaction.
	pub fn try_get_package_local_with_txn<Txn>(
		&self,
		txn: &Txn,
		hash: PackageHash,
	) -> Result<Option<Package>>
	where
		Txn: lmdb::Transaction,
	{
		match txn.get(self.inner.database.packages, &hash.as_slice()) {
			Ok(value) => {
				let value = Package::deserialize(value)?;
				Ok(Some(value))
			},
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => bail!(error),
		}
	}
}

impl Cli {
	pub fn get_package_local(&self, hash: PackageHash) -> Result<Package> {
		let package = self
			.try_get_package_local(hash)?
			.with_context(|| format!(r#"Failed to find the package with hash "{hash}"."#))?;
		Ok(package)
	}

	pub fn get_package_local_with_txn<Txn>(&self, txn: &Txn, hash: PackageHash) -> Result<Package>
	where
		Txn: lmdb::Transaction,
	{
		let package = self
			.try_get_package_local_with_txn(txn, hash)?
			.with_context(|| format!(r#"Failed to find the package with hash "{hash}"."#))?;
		Ok(package)
	}

	pub fn try_get_package_local(&self, hash: PackageHash) -> Result<Option<Package>> {
		// Begin a read transaction.
		let txn = self.inner.database.env.begin_ro_txn()?;

		// Get the package.
		let maybe_package = self.try_get_package_local_with_txn(&txn, hash)?;

		Ok(maybe_package)
	}
}

impl Cli {
	pub fn get_package_source(&self, package_hash: PackageHash) -> Result<ArtifactHash> {
		let package = self.get_package_local(package_hash)?;
		let package_source = package.source;
		Ok(package_source)
	}

	/// Get the manifest for the package with the given package hash.
	pub async fn get_package_manifest(&self, package_hash: PackageHash) -> Result<Manifest> {
		let package_source_artifact_hash = self.get_package_source(package_hash)?;
		self.get_package_manifest_for_source(package_source_artifact_hash)
			.await
	}

	/// Get the manifest from the source of package.
	pub async fn get_package_manifest_for_source(
		&self,
		package_source_artifact_hash: ArtifactHash,
	) -> Result<Manifest> {
		// Get the source directory.
		let source_directory = self
			.get_artifact_local(package_source_artifact_hash)?
			.into_directory()
			.context("Expected a directory.")?;

		// Get the manifest artifact hash.
		let manifest_artifact_hash = source_directory
			.entries
			.get("tangram.json")
			.copied()
			.context("The package source does not contain a manifest.")?;

		// Get the manifest blob hash.
		let manifest_blob_hash = self
			.get_artifact_local(manifest_artifact_hash)?
			.as_file()
			.context("Expected the manifest to be a file.")?
			.blob;

		// Read the manifest.
		let mut manifest = self
			.get_blob(manifest_blob_hash)
			.await
			.context("Failed to get the manifest blob.")?;
		let mut manifest_bytes = Vec::new();
		manifest
			.read_to_end(&mut manifest_bytes)
			.await
			.context("Failed to read the manifest.")?;

		// Deserialize the manifest.
		let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
			.context(r#"Failed to parse the package manifest."#)?;

		Ok(manifest)
	}
}

impl Cli {
	pub fn get_package_entrypoint_path(
		&self,
		package_hash: PackageHash,
	) -> Result<Option<Utf8PathBuf>> {
		const ENTRYPOINT_FILE_NAMES: [&str; 2] = ["package.tg", "tangram.js"];

		// Get the package source directory.
		let source_hash = self
			.get_package_source(package_hash)
			.context("Failed to get the package source.")?;
		let source_directory = self
			.get_artifact_local(source_hash)
			.context("Failed to get the package source.")?
			.into_directory()
			.context("The package source must be a directory.")?;

		// Get the entrypoint.
		let entrypoint = ENTRYPOINT_FILE_NAMES
			.into_iter()
			.find(|file_name| source_directory.entries.contains_key(*file_name))
			.map(Utf8PathBuf::from);

		Ok(entrypoint)
	}
}
