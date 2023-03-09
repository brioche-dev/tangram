use super::{Hash, Operation};
use crate::Instance;
use anyhow::Result;
use lmdb::Transaction;

impl Instance {
	/// Add an operation to the database.
	pub fn add_operation(&self, operation: &Operation) -> Result<Hash> {
		// Begin a write transaction.
		let mut txn = self.database.env.begin_rw_txn()?;

		// Serialize and hash the operation.
		let (hash, value) = operation.serialize_to_vec_and_hash();

		// Add the operation.
		txn.put(
			self.database.operations,
			&hash.as_slice(),
			&value,
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		Ok(hash)
	}
}
