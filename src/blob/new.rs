use super::{Blob, Data, Kind};
use crate::{
	block::Block,
	error::{Error, Result, WrapErr},
	instance::Instance,
};
use async_recursion::async_recursion;
use futures::{stream::FuturesOrdered, TryStreamExt};
use num_traits::ToPrimitive;
use std::path::Path;
use tokio::io::AsyncReadExt;

const MAX_BRANCH_CHILDREN: usize = 1024;

const MAX_LEAF_BLOCK_DATA_SIZE: usize = 262_144;

impl Blob {
	#[async_recursion]
	pub async fn new(tg: &Instance, children: Vec<Block>) -> Result<Self> {
		match children.len() {
			0 => {
				// Create the kind.
				let kind = Kind::Leaf(0);

				// Create the block.
				let block = Block::empty()?;

				// Create the blob.
				let blob = Self { block, kind };

				Ok(blob)
			},
			1 => {
				// Get the block.
				let block = children.into_iter().next().unwrap();

				// Get the size.
				let size = block.data_size(tg).await?.to_u64().unwrap();

				// Create the blob.
				let blob = Self {
					block,
					kind: Kind::Leaf(size),
				};

				Ok(blob)
			},
			_ => {
				// Get the sizes.
				let sizes = children
					.iter()
					.map(|block| async move {
						let blob = Blob::with_block(tg, block.clone()).await?;
						Ok::<_, Error>((blob.id(), blob.size()))
					})
					.collect::<FuturesOrdered<_>>()
					.try_collect::<Vec<_>>()
					.await?;

				// Create the data.
				let data = Data {
					sizes: sizes.clone(),
				};

				// Serialize the data.
				let mut bytes = Vec::new();
				data.serialize(&mut bytes).unwrap();
				let data = bytes;

				// Create the block.
				let block = Block::with_children_and_data(children, &data)?;

				// Create the blob.
				let sizes = sizes
					.into_iter()
					.map(|(id, size)| (Block::with_id(id), size))
					.collect();
				let kind = Kind::Branch(sizes);
				let blob = Self { block, kind };

				Ok(blob)
			},
		}
	}

	pub async fn with_bytes(tg: &Instance, bytes: &[u8]) -> Result<Self> {
		let block = Block::with_data(bytes)?;
		let blob = Self::new(tg, vec![block]).await?;
		Ok(blob)
	}

	pub async fn with_path(tg: &Instance, path: &Path) -> Result<Self> {
		// Open the file.
		let mut file = tokio::fs::File::open(path)
			.await
			.wrap_err("Failed to open the file.")?;

		// Create the blocks.
		let mut blocks = Vec::new();
		let size = file
			.metadata()
			.await
			.wrap_err("Failed to get the file's metadata.")?
			.len();
		let mut position = 0;
		let mut bytes = vec![0u8; MAX_LEAF_BLOCK_DATA_SIZE];
		while position < size {
			let n = std::cmp::min(size - position, MAX_LEAF_BLOCK_DATA_SIZE.to_u64().unwrap());
			let data = &mut bytes[..n.to_usize().unwrap()];
			file.read_exact(data).await?;
			position += n;
			let block = Block::with_data(data)?;
			block.store(tg).await?;
			if blocks.len() == MAX_BRANCH_CHILDREN {
				let blob = Self::new(tg, blocks).await?;
				blocks = vec![blob.block];
			}
			blocks.push(block);
		}

		// Create the blob.
		let blob = Self::new(tg, blocks)
			.await
			.wrap_err("Failed to create the blob.")?;

		Ok(blob)
	}
}
