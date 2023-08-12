use super::Block;
use crate::{
	error::{Error, Result},
	instance::Instance,
};
use lmdb::Transaction;

impl Block {
	/// Store this block.
	pub(crate) async fn store(&self, tg: &Instance) -> Result<()> {
		tokio::task::spawn_blocking({
			let block = self.clone();
			let tg = tg.clone();
			move || {
				// Start a write transaction.
				let mut txn = tg.store.env.begin_rw_txn()?;

				// Add the block and its children to the store recursively.
				let mut queue = vec![block];
				while let Some(mut block) = queue.pop() {
					// If the block is loaded, then unload it block and store the bytes.
					if let Some(bytes) = block.bytes.take() {
						txn.put(
							tg.store.blocks,
							&block.id().as_bytes(),
							&bytes.as_ref(),
							lmdb::WriteFlags::empty(),
						)?;
					}

					// If the block has children, then remove and enqueue them.
					if let Some(children) = block.children.take() {
						queue.extend(children.into_values());
					}
				}

				// Commit the transaction.
				txn.commit()?;

				Ok(())
			}
		})
		.await
		.map_err(Error::other)?
	}
}
