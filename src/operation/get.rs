use super::{Hash, Operation};
use crate::{
	error::{Result, WrapErr},
	instance::Instance,
	value::Value,
};

impl Operation {
	pub async fn get(tg: &Instance, hash: Hash) -> Result<Self> {
		let operation = Self::try_get(tg, hash)
			.await?
			.wrap_err_with(|| format!(r#"Failed to get the operation with hash "{hash}"."#))?;
		Ok(operation)
	}

	pub async fn try_get(tg: &Instance, hash: Hash) -> Result<Option<Self>> {
		// Attempt to get the operation locally.
		if let Some(operation) = Self::try_get_local(tg, hash).await? {
			return Ok(Some(operation));
		}

		// Attempt to get the operation from the API.
		let data = tg.api_client().try_get_operation(hash).await.ok().flatten();
		if let Some(data) = data {
			let operation = Operation::add(tg, data).await?;
			return Ok(Some(operation));
		}

		Ok(None)
	}

	pub async fn get_local(tg: &Instance, hash: Hash) -> Result<Self> {
		let operation = Self::try_get_local(tg, hash)
			.await?
			.wrap_err_with(|| format!(r#"Failed to find the operation with hash "{hash}"."#))?;
		Ok(operation)
	}

	pub async fn try_get_local(tg: &Instance, hash: Hash) -> Result<Option<Self>> {
		// Get the operation from the database.
		let Some(operation) = tg.database.try_get_operation(hash)? else {
			return Ok(None);
		};

		// Create the operation from the data.
		let operation = Self::from_data(tg, hash, operation).await?;

		Ok(Some(operation))
	}

	pub async fn get_operation_output(tg: &Instance, hash: Hash) -> Result<Option<Value>> {
		// Get the operation output from the database.
		let Some(output) = tg.database.try_get_operation_output(hash)? else {
			return Ok(None);
		};

		// Create the output from the data.
		let output = Value::from_data(tg, output).await?;

		Ok(Some(output))
	}
}
