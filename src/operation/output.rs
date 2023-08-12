use super::Operation;
use crate::{
	error::Result,
	instance::Instance,
	value::{self, Value},
};
use lmdb::Transaction;

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

	#[allow(clippy::unused_async)]
	pub async fn try_get_output_local(&self, tg: &Instance) -> Result<Option<Value>> {
		let data = {
			// Begin a read transaction.
			let txn = tg.database.env.begin_ro_txn()?;

			// Get the output from the database.
			let data = match txn.get(tg.database.outputs, &self.id().as_bytes()) {
				Ok(data) => data,
				Err(lmdb::Error::NotFound) => return Ok(None),
				Err(error) => return Err(error.into()),
			};

			value::Data::deserialize(data)?
		};

		let output = Value::from_data(tg, data).await?;

		Ok(Some(output))
	}

	#[allow(clippy::unused_async)]
	pub async fn set_output_local(&self, tg: &Instance, output: &Value) -> Result<()> {
		// Serialize the output data.
		let mut bytes = Vec::new();
		output.to_data().serialize(&mut bytes).unwrap();

		// Begin a write transaction.
		let mut txn = tg.database.env.begin_rw_txn()?;

		// Add the output to the database.
		txn.put(
			tg.database.outputs,
			&self.id().as_bytes(),
			&bytes,
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		Ok(())
	}
}
