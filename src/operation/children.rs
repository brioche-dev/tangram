use super::OperationHash;
use crate::Cli;
use anyhow::Result;
use lmdb::{Cursor, Transaction};

impl Cli {
	/// Add a run to the database.
	pub fn add_operation_child(
		&self,
		parent_operation_hash: OperationHash,
		child_operation_hash: OperationHash,
	) -> Result<()> {
		// Begin a write transaction.
		let mut txn = self.state.database.env.begin_rw_txn()?;

		// Add the child.
		txn.put(
			self.state.database.operation_children,
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
		operation_hash: OperationHash,
	) -> Result<impl Iterator<Item = Result<OperationHash>>>
	where
		Txn: lmdb::Transaction,
	{
		// Open a readonly cursor.
		let mut cursor = txn.open_ro_cursor(self.state.database.operation_children)?;

		// Get the children.
		let children = cursor
			.iter_dup_of(operation_hash.as_slice())
			.into_iter()
			.map(|value| match value {
				Ok((_, value)) => {
					let value = buffalo::from_slice(value)?;
					Ok(value)
				},
				Err(error) => Err(error.into()),
			});

		Ok(children)
	}
}
