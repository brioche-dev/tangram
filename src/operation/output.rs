use super::Operation;
use crate::{
	error::{Result, WrapErr},
	instance::Instance,
	value::Value,
};

impl Operation {
	pub async fn output(&self, tg: &Instance) -> Result<Option<Value>> {
		if let Some(output) = tg.database.get_operation_output(self.hash()).await? {
			let output = Value::from_data(tg, output).await?;
			return Ok(Some(output));
		}
		Ok(None)
	}

	pub async fn set_output(&self, tg: &Instance, value: &Value) -> Result<()> {
		let hash = self.hash();
		let data = value.to_data();
		tg.database
			.set_operation_output(self.hash(), &data)
			.await
			.wrap_err_with(|| {
				format!(
					r#"Failed tot set the operation output for the operation with hash "{hash}"."#
				)
			})?;
		Ok(())
	}
}
