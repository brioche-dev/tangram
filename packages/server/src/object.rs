use super::Server;
use bytes::Bytes;
use futures::{stream, StreamExt, TryStreamExt};
use tangram_client as tg;
use tangram_error::{Error, Result, WrapErr};
use tg::object;

impl Server {
	pub async fn get_object_exists(&self, id: &object::Id) -> Result<bool> {
		// Check if the object exists in the database.
		if self.get_object_exists_from_database(id)? {
			return Ok(true);
		}

		// Check if the object exists in the remote.
		if let Ok(true) = self.get_object_exists_from_remote(id).await {
			return Ok(true);
		}

		Ok(false)
	}

	pub fn get_object_exists_from_database(&self, id: &object::Id) -> Result<bool> {
		self.inner.database.get_object_exists(id)
	}

	async fn get_object_exists_from_remote(&self, id: &object::Id) -> Result<bool> {
		if let Some(remote) = self.inner.remote.as_ref() {
			if remote.get_object_exists(id).await? {
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

		// Attempt to get the object from the remote.
		if let Ok(Some(bytes)) = self.try_get_object_from_remote(id).await {
			return Ok(Some(bytes));
		};

		Ok(None)
	}

	pub fn try_get_object_from_database(&self, id: &object::Id) -> Result<Option<Bytes>> {
		self.inner.database.get_object(id)
	}

	async fn try_get_object_from_remote(&self, id: &object::Id) -> Result<Option<Bytes>> {
		let Some(remote) = self.inner.remote.as_ref() else {
			return Ok(None);
		};

		// Get the object from the remote.
		let Some(bytes) = remote.try_get_object(id).await? else {
			return Ok(None);
		};

		// Add the object to the database.
		self.inner.database.put_object(id, &bytes)?;

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

		// Add the object to the database.
		self.inner.database.put_object(id, bytes)?;

		Ok(Ok(()))
	}
}
