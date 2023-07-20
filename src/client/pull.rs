use super::Client;
use crate::{
	block::{self, Block},
	error::Result,
	instance::Instance,
	return_error,
};
use async_recursion::async_recursion;
use futures::{stream::FuturesUnordered, TryStreamExt};
use std::io::Cursor;
use tokio::io::AsyncReadExt;

impl Client {
	/// Pull a block.
	#[async_recursion]
	#[must_use]
	pub async fn pull(&self, tg: &Instance, block: Block) -> Result<()> {
		// If the block is in this instance's database, then return.
		if block.is_local(tg)? {
			return Ok(());
		}

		// Otherwise, get the block's bytes.
		let Some(mut reader) = self.try_get_block(block.id()).await? else {
			return_error!(r#"Failed to get the block "{block}"."#);
		};
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes).await?;

		// Pull the block's children.
		let mut reader = block::Reader::new(Cursor::new(bytes));
		reader
			.children()?
			.into_iter()
			.map(|child| self.pull(tg, child))
			.collect::<FuturesUnordered<_>>()
			.try_collect()
			.await?;
		let bytes = reader.into_inner().into_inner();

		// Add the block.
		Block::add(tg, block.id(), bytes)?;

		Ok(())
	}
}
