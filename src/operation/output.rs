use super::Operation;
use crate::{error::Result, instance::Instance, value::Value};

impl Operation {
	pub async fn try_get_output(&self, tg: &Instance) -> Result<Option<Value>> {
		// Attempt to get the output locally.
		if let Some(output) = self.try_get_output_local(tg).await? {
			return Ok(Some(output));
		};

		// // Attempt to get the output from the API.
		// let output = tg.api_client.try_get_output(tg, self).await.ok().flatten();
		// if let Some(output) = output {
		// 	// Add the operation output to the database.
		// 	self.set_output_local(tg, &output)?;

		// 	return Ok(Some(output));
		// }

		Ok(None)
	}

	pub async fn try_get_output_local(&self, tg: &Instance) -> Result<Option<Value>> {
		let data = {
			let connection = tg.get_database_connection()?;
			let mut statement =
				connection.prepare_cached("select value from outputs where id = ?")?;
			let mut rows = statement.query(rusqlite::params![self.block().id()])?;
			let Some(row) = rows.next()? else {
				return Ok(None);
			};
			let data: Vec<u8> = row.get(0)?;
			data
		};
		let output = Value::from_bytes(tg, &data).await?;
		Ok(Some(output))
	}

	pub fn set_output_local(&self, tg: &Instance, output: &Value) -> Result<()> {
		let connection = tg.get_database_connection()?;
		let mut statement =
			connection.prepare_cached("insert into outputs (id, value) values (?, ?)")?;
		statement.execute(rusqlite::params![self.block().id(), output.to_bytes()?])?;
		Ok(())
	}
}
