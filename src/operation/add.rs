use super::{Data, Hash, Operation};
use crate::{error::Result, instance::Instance};

impl Operation {
	pub async fn add(tg: &Instance, data: Data) -> Result<Self> {
		// Serialize and hash the operation data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let hash = Hash(crate::hash::Hash::new(&bytes));

		// Add the operation to the database.
		let hash = tg.database.add_operation(hash, &bytes)?;

		// Create the operation.
		let operation = Self::from_data(tg, hash, data).await?;

		Ok(operation)
	}
}
