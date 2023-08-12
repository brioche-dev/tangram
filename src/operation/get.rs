use super::{Data, Operation};
use crate::{
	block::Block,
	error::{Result, WrapErr},
	instance::Instance,
};

impl Operation {
	pub async fn with_block(tg: &Instance, block: Block) -> Result<Self> {
		let id = block.id();
		let operation = Self::try_with_block(tg, block)
			.await?
			.wrap_err_with(|| format!(r#"Failed to get operation "{id}"."#))?;
		Ok(operation)
	}

	pub async fn try_with_block(tg: &Instance, block: Block) -> Result<Option<Self>> {
		// Get the data.
		let Some(data) = block.try_get_data(tg).await? else {
			return Ok(None);
		};

		// Deserialize the data.
		let data = Data::deserialize(&*data)?;

		// Create the operation from the data.
		let operation = Self::from_data(tg, block, data).await?;

		Ok(Some(operation))
	}
}
