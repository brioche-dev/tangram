use super::{Data, Operation};
use crate::{
	block::Block,
	error::{Result, WrapErr},
	instance::Instance,
};

impl Operation {
	pub async fn get(tg: &Instance, block: Block) -> Result<Self> {
		let operation = Self::try_get(tg, block)
			.await?
			.wrap_err_with(|| format!(r#"Failed to get the operation with block "{block}"."#))?;
		Ok(operation)
	}

	pub async fn try_get(tg: &Instance, block: Block) -> Result<Option<Self>> {
		// Get the data.
		let Some(data) = block.try_get_data(tg).await? else {
			return Ok(None);
		};

		// Deserialize the data.
		let data = Data::deserialize(data.as_slice())?;

		// Create the operation from the data.
		let operation = Self::from_data(tg, block, data).await?;

		Ok(Some(operation))
	}
}
