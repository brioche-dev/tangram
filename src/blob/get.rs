use super::{Blob, Data, Kind};
use crate::{
	block::Block,
	error::{Result, WrapErr},
	instance::Instance,
};
use async_recursion::async_recursion;
use num_traits::ToPrimitive;

impl Blob {
	#[async_recursion]
	pub async fn get(tg: &'async_recursion Instance, block: Block) -> Result<Self> {
		let artifact = Self::try_get(tg, block)
			.await?
			.wrap_err_with(|| format!(r#"Failed to get the blob with block "{block}"."#))?;
		Ok(artifact)
	}

	pub async fn try_get(tg: &Instance, block: Block) -> Result<Option<Self>> {
		// Get the children.
		let Some(children) = block.try_get_children(tg).await? else {
			return Ok(None);
		};

		let blob = if children.is_empty() {
			// If the block has no children, then it is a leaf.

			// Get the size.
			let size = block.data_size(tg).await?.to_u64().unwrap();

			Self {
				block,
				kind: Kind::Leaf(size),
			}
		} else {
			// Otherwise, it is a branch.

			// Get the data.
			let data = block.data(tg).await?;

			// Deserialize the data.
			let data = Data::deserialize(data.as_slice())?;

			// Create the blob from the data.
			Self {
				block,
				kind: Kind::Branch(data.sizes),
			}
		};

		Ok(Some(blob))
	}
}
