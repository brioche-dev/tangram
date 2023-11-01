use super::Server;
use bytes::Bytes;
use futures::{stream, StreamExt, TryStreamExt};
use lmdb::Transaction;
use tangram_client as tg;
use tg::{object, Error, Result, Wrap, WrapErr};

impl Server {
	pub async fn get_object_exists(&self, id: &object::Id) -> Result<bool> {
		// Check if the object exists in the database.
		if self.get_object_exists_from_database(id)? {
			return Ok(true);
		}

		// Check if the object exists in the parent.
		if let Ok(true) = self.get_object_exists_from_parent(id).await {
			return Ok(true);
		}

		Ok(false)
	}

	pub fn get_object_exists_from_database(&self, id: &object::Id) -> Result<bool> {
		let txn = self
			.inner
			.database
			.env
			.begin_ro_txn()
			.wrap_err("Failed to create the transaction.")?;
		match txn.get(self.inner.database.objects, &id.to_bytes()) {
			Ok(_) => Ok(true),
			Err(lmdb::Error::NotFound) => Ok(false),
			Err(error) => Err(error.wrap("Failed to get the object.")),
		}
	}

	async fn get_object_exists_from_parent(&self, id: &object::Id) -> Result<bool> {
		if let Some(parent) = self.inner.parent.as_ref() {
			if parent.get_object_exists(id).await? {
				return Ok(true);
			}
		}
		Ok(false)
	}

	pub async fn get_object(&self, id: &object::Id) -> Result<Bytes> {
		self.try_get_object(id)
			.await?
			.wrap_err("Failed to get the object.")
	}

	pub async fn try_get_object(&self, id: &object::Id) -> Result<Option<Bytes>> {
		// Attempt to get the object from the database.
		if let Some(bytes) = self.try_get_object_from_database(id)? {
			return Ok(Some(bytes));
		};

		// Attempt to get the object from the parent.
		if let Ok(Some(bytes)) = self.try_get_object_from_parent(id).await {
			return Ok(Some(bytes));
		};

		Ok(None)
	}

	pub fn try_get_object_from_database(&self, id: &object::Id) -> Result<Option<Bytes>> {
		let txn = self
			.inner
			.database
			.env
			.begin_ro_txn()
			.wrap_err("Failed to create the transaction.")?;
		let data = match txn.get(self.inner.database.objects, &id.to_bytes()) {
			Ok(data) => data,
			Err(lmdb::Error::NotFound) => return Ok(None),
			Err(error) => return Err(error.wrap("Failed to get the object.")),
		};
		let data = Bytes::copy_from_slice(data);
		Ok(Some(data))
	}

	async fn try_get_object_from_parent(&self, id: &object::Id) -> Result<Option<Bytes>> {
		let Some(parent) = self.inner.parent.as_ref() else {
			return Ok(None);
		};

		// Get the object from the parent.
		let Some(bytes) = parent.try_get_object(id).await? else {
			return Ok(None);
		};

		// Create a write transaction.
		let mut txn = self
			.inner
			.database
			.env
			.begin_rw_txn()
			.wrap_err("Failed to create the transaction.")?;

		// Add the object to the database.
		txn.put(
			self.inner.database.objects,
			&id.to_bytes(),
			&bytes,
			lmdb::WriteFlags::empty(),
		)
		.wrap_err("Failed to put the object.")?;

		// Commit the transaction.
		txn.commit().wrap_err("Failed to commit the transaction.")?;

		Ok(Some(bytes))
	}

	pub async fn try_put_object(
		&self,
		id: &object::Id,
		bytes: &Bytes,
	) -> Result<Result<(), Vec<object::Id>>> {
		// Deserialize the object.
		let data = object::Data::deserialize(id.kind(), bytes)
			.wrap_err("Failed to serialize the data.")?;

		// Check if there are any missing children.
		let missing_children = stream::iter(data.children())
			.map(Ok)
			.try_filter_map(|id| async move {
				let exists = self.get_object_exists(&id).await?;
				Ok::<_, Error>(if exists { None } else { Some(id) })
			})
			.try_collect::<Vec<_>>()
			.await?;
		if !missing_children.is_empty() {
			return Ok(Err(missing_children));
		}

		// Create a write transaction.
		let mut txn = self
			.inner
			.database
			.env
			.begin_rw_txn()
			.wrap_err("Failed to create the transaction.")?;

		// Add the object to the database.
		txn.put(
			self.inner.database.objects,
			&id.to_bytes(),
			&bytes,
			lmdb::WriteFlags::empty(),
		)
		.wrap_err("Failed to put the object.")?;

		// Commit the transaction.
		txn.commit().wrap_err("Failed to commit the transaction.")?;

		Ok(Ok(()))
	}
}
