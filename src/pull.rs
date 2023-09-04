use super::Client;
use crate::{
	block::{self, Block},
	error::Result,
	id::Id,
	return_error,
	server::Server,
};
use async_recursion::async_recursion;
use futures::{stream::FuturesUnordered, TryStreamExt};
use tokio::io::AsyncReadExt;

impl Client {
	/// Pull a block.
	#[async_recursion]
	#[must_use]
	pub async fn pull(&self, tg: &Server, id: Id) -> Result<Block> {
		// If the block is stored locally, then return.
		let block = Block::with_id(id);
		if block.exists_local(tg).await? {
			return Ok(block);
		}

		// Otherwise, get the block's bytes.
		let Some(mut reader) = self.try_get_block(id).await? else {
			return_error!(r#"Failed to get block "{id}"."#);
		};
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes).await?;

		// Pull the block's children.
		let mut reader = block::Reader::with_bytes(&bytes);
		let children = reader
			.children()?
			.into_iter()
			.map(|child| self.pull(tg, child.id()))
			.collect::<FuturesUnordered<_>>()
			.try_collect()
			.await?;

		// Create the block.
		let block = Block::with_id_and_bytes(tg, id, bytes.into())?;

		Ok(block)
	}
}
