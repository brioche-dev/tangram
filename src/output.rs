use crate as tg;
use crate::{error::Error, id::Id, rid::Rid};
use crate::{
	error::{Result, WrapErr},
	server::Server,
	value::{self, Value},
};
use lmdb::Transaction;
use tangram_serialize::Deserialize;

pub struct Output {
	run: Rid,
	value: Option<Id>,
	error: Option<Error>,
}

impl Build {
	pub async fn try_get_output(&self, tg: &Server) -> Result<Option<Value>> {
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

	pub async fn try_get_output_local(&self, tg: &Server) -> Result<Option<Value>> {
		// Begin a read transaction.
		let txn = tg.database.env.begin_ro_txn()?;

		// Get the output from the database.
		let data = match txn.get(tg.database.outputs, &self.id().as_bytes()) {
			Ok(data) => data,
			Err(lmdb::Error::NotFound) => return Ok(None),
			Err(error) => return Err(error.into()),
		};

		let output = tg::Value::deserialize(data)?;

		Ok(Some(output))
	}

	pub async fn set_output_local(&self, tg: &Server, output: &tg::Value) -> Result<()> {
		tokio::task::spawn_blocking({
			let operation = self.clone();
			let tg = tg.clone();
			move || {
				// Serialize the output data.
				let mut bytes = Vec::new();
				output.to_data().serialize(&mut bytes).unwrap();

				// Begin a write transaction.
				let mut txn = tg.database.env.begin_rw_txn()?;

				// Add the output to the database.
				txn.put(
					tg.database.outputs,
					&operation.id().as_bytes(),
					&bytes,
					lmdb::WriteFlags::empty(),
				)?;

				// Commit the transaction.
				txn.commit()?;

				Ok::<_, crate::error::Error>(())
			}
		})
		.await
		.map_err(crate::error::Error::other)
		.wrap_err("Failed to join the store task.")?
		.wrap_err("Failed to store the block.")?;
		Ok(())
	}
}
