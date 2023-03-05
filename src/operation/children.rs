use super::Hash;
use crate::Instance;
use anyhow::Result;
use lmdb::{Cursor, Transaction};

impl Instance {
	/// Add a run to the database.
	pub fn add_operation_child(
		&self,
		parent_operation_hash: Hash,
		child_operation_hash: Hash,
	) -> Result<()> {
		// Begin a write transaction.
		let mut txn = self.database.env.begin_rw_txn()?;

		// Add the child.
		txn.put(
			self.database.operation_children,
			&parent_operation_hash.as_slice(),
			&child_operation_hash.as_slice(),
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		Ok(())
	}

	/// Get the children for an operation.
	pub fn get_operation_children_with_txn<Txn>(
		&self,
		txn: &Txn,
		operation_hash: Hash,
	) -> Result<impl Iterator<Item = Result<Hash>>>
	where
		Txn: lmdb::Transaction,
	{
		// Open a readonly cursor.
		let mut cursor = txn.open_ro_cursor(self.database.operation_children)?;

		// Get the children.
		let children = cursor
			.iter_dup_of(operation_hash.as_slice())
			.into_iter()
			.map(|value| {
				let (_, value) = value?;
				let value = buffalo::from_slice(value)?;
				Ok(value)
			});

		Ok(children)
	}
}
