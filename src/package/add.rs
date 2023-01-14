use super::{Package, PackageHash};
use crate::{artifact::ArtifactHash, Cli};
use anyhow::{bail, Result};
use lmdb::Transaction;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AddPackageOutcome {
	Added {
		package_hash: PackageHash,
	},
	MissingSource {
		source: ArtifactHash,
	},
	MissingDependencies {
		dependencies: Vec<(String, PackageHash)>,
	},
}

impl Cli {
	/// Add a pacakge after ensuring all its references are present.
	pub fn try_add_package(&self, package: &Package) -> Result<AddPackageOutcome> {
		// Ensure the package's source is present.
		let source = package.source;
		let exists = self.artifact_exists_local(source)?;
		if !exists {
			return Ok(AddPackageOutcome::MissingSource { source });
		}

		// Ensure all the package's dependencies are present.
		let mut dependencies = Vec::new();
		for (name, package_hash) in &package.dependencies {
			let package_hash = *package_hash;
			let exists = self.package_exists_local(package_hash)?;
			if !exists {
				dependencies.push((name.clone(), package_hash));
			}
		}
		if !dependencies.is_empty() {
			return Ok(AddPackageOutcome::MissingDependencies { dependencies });
		}

		// Hash the package.
		let package_hash = package.hash();

		// Serialize the package.
		let value = package.serialize_to_vec();

		// Begin a write transaction.
		let mut txn = self.inner.database.env.begin_rw_txn()?;

		// Add the package to the database.
		match txn.put(
			self.inner.database.packages,
			&package_hash.as_slice(),
			&value,
			lmdb::WriteFlags::NO_OVERWRITE,
		) {
			Ok(_) | Err(lmdb::Error::KeyExist) => Ok(()),
			Err(error) => Err(error),
		}?;

		// Commit the transaction.
		txn.commit()?;

		Ok(AddPackageOutcome::Added { package_hash })
	}
}

impl Cli {
	pub fn add_package(&self, package: &Package) -> Result<PackageHash> {
		match self.try_add_package(package)? {
			AddPackageOutcome::Added { package_hash } => Ok(package_hash),
			_ => bail!("Failed to add the package."),
		}
	}
}
