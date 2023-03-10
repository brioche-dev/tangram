use super::{Hash, Instance};
use crate::error::{bail, Context, Result};
use lmdb::Transaction;

impl crate::Instance {
	/// Get a package instance from the database. This method returns an error if the package instance is not found.
	pub fn package_instance_exists_local(&self, package_instance_hash: Hash) -> Result<bool> {
		// Begin a read transaction.
		let txn = self.database.env.begin_ro_txn()?;

		let exists = match txn.get(
			self.database.package_instances,
			&package_instance_hash.as_slice(),
		) {
			Ok(_) => Ok::<_, anyhow::Error>(true),
			Err(lmdb::Error::NotFound) => Ok(false),
			Err(error) => Err(error.into()),
		}?;

		Ok(exists)
	}

	/// Try to get a package instance from the database with the given transaction.
	pub fn try_get_package_instance_local_with_txn<Txn>(
		&self,
		txn: &Txn,
		hash: Hash,
	) -> Result<Option<Instance>>
	where
		Txn: lmdb::Transaction,
	{
		match txn.get(self.database.package_instances, &hash.as_slice()) {
			Ok(value) => {
				let value = Instance::deserialize(value)?;
				Ok(Some(value))
			},
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => bail!(error),
		}
	}
}

impl crate::Instance {
	pub fn get_package_instance_local(&self, hash: Hash) -> Result<Instance> {
		let package_instance = self
			.try_get_package_instance_local(hash)?
			.with_context(|| {
				format!(r#"Failed to find the package instance with hash "{hash}"."#)
			})?;
		Ok(package_instance)
	}

	pub fn get_package_instance_local_with_txn<Txn>(
		&self,
		txn: &Txn,
		hash: Hash,
	) -> Result<Instance>
	where
		Txn: lmdb::Transaction,
	{
		let package_instance = self
			.try_get_package_instance_local_with_txn(txn, hash)?
			.with_context(|| format!(r#"Failed to find the package with hash "{hash}"."#))?;
		Ok(package_instance)
	}

	pub fn try_get_package_instance_local(&self, hash: Hash) -> Result<Option<Instance>> {
		// Begin a read transaction.
		let txn = self.database.env.begin_ro_txn()?;

		// Get the package instance.
		let maybe_package_instance = self.try_get_package_instance_local_with_txn(&txn, hash)?;

		Ok(maybe_package_instance)
	}
}
