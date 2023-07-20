use super::{block::TryAddBlockOutcome, Client};
use crate::{
	block::Block,
	error::{return_error, Error, Result},
	instance::Instance,
};
use async_recursion::async_recursion;
use futures::{stream::FuturesUnordered, TryStreamExt};
use std::{io::Cursor, sync::Arc};

impl Client {
	/// Push a block.
	#[async_recursion]
	#[must_use]
	pub async fn push(&self, tg: &Instance, block: Block) -> Result<()> {
		// Attempt to add the block.
		let bytes: Arc<[u8]> = block.bytes(tg).await?.into();
		let reader = Cursor::new(bytes.clone());
		let outcome = self.try_add_block(block.id(), reader).await?;

		// If the block was added, then return. Otherwise, push the missing children.
		match outcome {
			TryAddBlockOutcome::Added => return Ok(()),
			TryAddBlockOutcome::MissingChildren(children) => {
				children
					.into_iter()
					.map(|id| async move {
						let child = Block::with_id(id);
						self.push(tg, child).await?;
						Ok::<_, Error>(())
					})
					.collect::<FuturesUnordered<_>>()
					.try_collect()
					.await?;
			},
		}

		// Attempt to add the block again. This time, return an error if there are missing children.
		let reader = Cursor::new(bytes.clone());
		let outcome = self.try_add_block(block.id(), reader).await?;
		match outcome {
			TryAddBlockOutcome::Added => {},
			TryAddBlockOutcome::MissingChildren(_) => {
				return_error!("Failed to push the block.");
			},
		}

		Ok(())
	}
}
