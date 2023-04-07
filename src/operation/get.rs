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
		// Attempt to get the operation from the database.
		if let Some(operation) = Self::try_get_local(tg, hash).await? {
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
		// Get the serialized operation from the database.
		let Some(operation) = tg.database.try_get_operation(hash).await? else {
			return Ok(None);
		};

		// Create the operation from the serialized operation.
		let operation = Self::from_data(tg, hash, operation).await?;

		Ok(Some(operation))
	}

	pub async fn get_operation_output(tg: &Instance, hash: Hash) -> Result<Option<Value>> {
		// Get the serialized operation output from the database.
		let Some(output) = tg.database.get_operation_output(hash).await? else {
			return Ok(None);
		};

		// Create the output from the serialized output.
		let output = Value::from_data(tg, output).await?;

		Ok(Some(output))
	}
}
