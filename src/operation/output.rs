use super::Operation;
use crate::{
	error::{Result, WrapErr},
	instance::Instance,
	value::Value,
};

impl Operation {
	pub async fn try_get_output(&self, tg: &Instance) -> Result<Option<Value>> {
		// Attempt to get the output locally.
		if let Some(output) = self.try_get_output_local(tg).await? {
			return Ok(Some(output));
		}

		// Attempt to get the output from the API.
		let data = tg
			.api_client()
			.try_get_operation_output(self.hash())
			.await
			.ok()
			.flatten();
		if let Some(data) = data {
			// Add the operation output to the database.
			tg.database.set_operation_output(self.hash(), &data)?;

			// Get the value.
			let output = Value::from_data(tg, data).await?;

			return Ok(Some(output));
		}

		Ok(None)
	}

	pub async fn try_get_output_local(&self, tg: &Instance) -> Result<Option<Value>> {
		if let Some(output) = tg.database.try_get_operation_output(self.hash())? {
			let output = Value::from_data(tg, output).await?;
			return Ok(Some(output));
		}
		Ok(None)
	}

	pub fn set_output_local(&self, tg: &Instance, value: &Value) -> Result<()> {
		let hash = self.hash();
		let data = value.to_data();
		tg.database
			.set_operation_output(self.hash(), &data)
			.wrap_err_with(|| {
				format!(
					r#"Failed tot set the operation output for the operation with hash "{hash}"."#
				)
			})?;
		Ok(())
	}
}
