use super::{Hash, Operation};
use crate::Instance;
use anyhow::{bail, Context, Result};

impl Instance {
	pub fn get_operation_local(&self, hash: Hash) -> Result<Operation> {
		let operation = self
			.try_get_operation_local(hash)?
			.with_context(|| format!(r#"Failed to find the operation with hash "{hash}"."#))?;
		Ok(operation)
	}

	pub fn get_operation_local_with_txn<Txn>(&self, txn: &Txn, hash: Hash) -> Result<Operation>
	where
		Txn: lmdb::Transaction,
	{
		let operation = self
			.try_get_operation_local_with_txn(txn, hash)?
			.with_context(|| format!(r#"Failed to find the operation with hash "{hash}"."#))?;
		Ok(operation)
	}

	pub fn try_get_operation_local(&self, hash: Hash) -> Result<Option<Operation>> {
		// Begin a read transaction.
		let txn = self.database.env.begin_ro_txn()?;

		// Get the operation.
		let maybe_operation = self.try_get_operation_local_with_txn(&txn, hash)?;

		Ok(maybe_operation)
	}

	/// Try to get an operation from the database with the given transaction.
	pub fn try_get_operation_local_with_txn<Txn>(
		&self,
		txn: &Txn,
		hash: Hash,
	) -> Result<Option<Operation>>
	where
		Txn: lmdb::Transaction,
	{
		match txn.get(self.database.operations, &hash.as_slice()) {
			Ok(value) => {
				let value = Operation::deserialize(value)?;
				Ok(Some(value))
			},
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => bail!(error),
		}
	}
}
