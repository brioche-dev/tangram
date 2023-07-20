use super::{Blob, Data, Kind};
use crate::{
	block::Block,
	error::{Error, Result},
	instance::Instance,
};
use async_recursion::async_recursion;
use futures::{stream::FuturesOrdered, TryStreamExt};
use num_traits::ToPrimitive;
use std::path::Path;
use tokio::io::AsyncReadExt;

const MAX_CHILDREN: usize = 1024;

const MAX_BLOCK_SIZE: usize = 262_144;

impl Blob {
	#[async_recursion]
	pub async fn new(tg: &Instance, children: Vec<Block>) -> Result<Self> {
		match children.len() {
			0 => {
				// Create the kind.
				let kind = Kind::Leaf(0);

				// Create the block.
				let block = Block::new(tg, Vec::new(), &[])?;

				// Create the blob.
				let blob = Self { block, kind };

				Ok(blob)
			},
			1 => {
				// Get the block.
				let block = children[0];

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
						let blob = Blob::get(tg, *block).await?;
						let size = blob.size();
						Ok::<_, Error>((*block, size))
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
				let block = Block::new(tg, children, &data)?;

				// Create the blob.
				let kind = Kind::Branch(sizes);
				let blob = Self { block, kind };

				Ok(blob)
			},
		}
	}

	pub async fn with_bytes(tg: &Instance, bytes: impl AsRef<[u8]>) -> Result<Self> {
		let block = Block::new(tg, vec![], bytes.as_ref())?;
		let blob = Self::new(tg, vec![block]).await?;
		Ok(blob)
	}

	pub async fn with_path(tg: &Instance, path: &Path) -> Result<Self> {
		// Open the file.
		let mut file = tokio::fs::File::open(path).await?;

		// Create the blocks.
		let mut blocks = Vec::new();
		let size = file.metadata().await?.len();
		let mut position = 0;
		let mut bytes = vec![0u8; MAX_BLOCK_SIZE];
		while position < size {
			let n = std::cmp::min(size - position, MAX_BLOCK_SIZE.to_u64().unwrap());
			let bytes = &mut bytes[..n.to_usize().unwrap()];
			file.read_exact(bytes).await?;
			position += n;
			let block = Block::new(tg, vec![], bytes)?;
			if blocks.len() == MAX_CHILDREN {
				let blob = Self::new(tg, blocks).await?;
				blocks = vec![blob.block];
			}
			blocks.push(block);
		}

		// Create the blob.
		let blob = Self::new(tg, blocks).await?;

		Ok(blob)
	}
}
