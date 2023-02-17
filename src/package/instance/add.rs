use super::{Hash, Instance};
use crate::{artifact, Cli};
use anyhow::{bail, Result};
use lmdb::Transaction;

/// The outcome of adding a package instance.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
	/// The package instance was added.
	Added { hash: Hash },

	/// The package was missing.
	MissingPackage { package_hash: artifact::Hash },

	/// Some dependencies were missing.
	MissingDependencies { dependencies: Vec<(String, Hash)> },
}

impl Cli {
	/// Add a pacakge after ensuring all its references are present.
	pub fn try_add_package_instance(&self, package_instance: &Instance) -> Result<Outcome> {
		// Ensure the package is present.
		let exists = self.artifact_exists_local(package_instance.package_hash)?;
		if !exists {
			return Ok(Outcome::MissingPackage {
				package_hash: package_instance.package_hash,
			});
		}

		// Ensure all the package instance's dependencies are present.
		let mut dependencies = Vec::new();
		for (name, dependency_hash) in &package_instance.dependencies {
			let dependency_hash = *dependency_hash;
			let exists = self.package_instance_exists_local(dependency_hash)?;
			if !exists {
				dependencies.push((name.clone(), dependency_hash));
			}
		}
		if !dependencies.is_empty() {
			return Ok(Outcome::MissingDependencies { dependencies });
		}

		// Hash the package instance.
		let hash = package_instance.hash();

		// Serialize the package instance.
		let value = package_instance.serialize_to_vec();

		// Begin a write transaction.
		let mut txn = self.database.env.begin_rw_txn()?;

		// Add the package instance to the database.
		match txn.put(
			self.database.package_instances,
			&hash.as_slice(),
			&value,
			lmdb::WriteFlags::NO_OVERWRITE,
		) {
			Ok(_) | Err(lmdb::Error::KeyExist) => Ok(()),
			Err(error) => Err(error),
		}?;

		// Commit the transaction.
		txn.commit()?;

		Ok(Outcome::Added { hash })
	}
}

impl Cli {
	pub fn add_package_instance(&self, package_instance: &Instance) -> Result<Hash> {
		match self.try_add_package_instance(package_instance)? {
			Outcome::Added { hash } => Ok(hash),
			_ => bail!("Failed to add the package instance."),
		}
	}
}
