use super::{Artifact, Data};
use crate::{
	block::Block,
	error::{Result, WrapErr},
	instance::Instance,
};
use async_recursion::async_recursion;

impl Artifact {
	#[async_recursion]
	pub async fn get(tg: &'async_recursion Instance, block: Block) -> Result<Self> {
		let artifact = Self::try_get(tg, block)
			.await?
			.wrap_err_with(|| format!(r#"Failed to get the artifact with block "{block}"."#))?;
		Ok(artifact)
	}

	pub async fn try_get(tg: &Instance, block: Block) -> Result<Option<Self>> {
		// Get the data.
		let Some(data) = block.try_get_data(tg).await? else {
			return Ok(None);
		};

		// Deserialize the data.
		let data = Data::deserialize(data.as_slice())?;

		// Create the artifact from the data.
		let artifact = Self::from_data(tg, block, data).await?;

		Ok(Some(artifact))
	}
}
