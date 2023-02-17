use super::Hash;
use crate::{value::Value, Cli};
use anyhow::Result;
use lmdb::Transaction;

impl Cli {
	/// Get the output for an operation from the database.
	pub fn get_operation_output(&self, operation_hash: Hash) -> Result<Option<Value>> {
		// Begin a read transaction.
		let txn = self.database.env.begin_ro_txn()?;

		// Get the output.
		let output = match txn.get(self.database.operation_outputs, &operation_hash.as_slice()) {
			Ok(value) => {
				let value = Value::deserialize(value)?;
				Ok::<_, anyhow::Error>(Some(value))
			},
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => Err(error.into()),
		}?;

		Ok(output)
	}

	/// Set the output for an operation in the database.
	pub fn set_operation_output(&self, operation_hash: Hash, value: &Value) -> Result<()> {
		// Begin a write transaction.
		let mut txn = self.database.env.begin_rw_txn()?;

		// Serialize the value.
		let value = value.serialize_to_vec();

		// Add the output to the database.
		txn.put(
			self.database.operation_outputs,
			&operation_hash.as_slice(),
			&value,
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		Ok(())
	}
}
