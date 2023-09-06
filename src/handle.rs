use crate::{
	error::{error, Result},
	return_error, value, Client, Error, Id, Kind, Server, Value, WrapErr,
};
use async_recursion::async_recursion;
use futures::{stream::FuturesUnordered, TryStreamExt};
use lmdb::Transaction;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Handle {
	id: Arc<std::sync::RwLock<Option<Id>>>,
	value: Arc<std::sync::RwLock<Option<Value>>>,
}

impl Handle {
	#[must_use]
	pub fn with_id(id: Id) -> Self {
		Self {
			id: Arc::new(std::sync::RwLock::new(Some(id))),
			value: Arc::new(std::sync::RwLock::new(None)),
		}
	}

	#[must_use]
	pub fn with_value(value: Value) -> Self {
		Self {
			id: Arc::new(std::sync::RwLock::new(None)),
			value: Arc::new(std::sync::RwLock::new(Some(value))),
		}
	}

	#[must_use]
	pub fn kind(&self) -> Kind {
		if let Some(id) = *self.id.read().unwrap() {
			return id.kind();
		}
		match self.value.read().unwrap().as_ref().unwrap() {
			Value::Null(_) => Kind::Null,
			Value::Bool(_) => Kind::Bool,
			Value::Number(_) => Kind::Number,
			Value::String(_) => Kind::String,
			Value::Bytes(_) => Kind::Bytes,
			Value::Relpath(_) => Kind::Relpath,
			Value::Subpath(_) => Kind::Subpath,
			Value::Blob(_) => Kind::Blob,
			Value::Directory(_) => Kind::Directory,
			Value::File(_) => Kind::File,
			Value::Symlink(_) => Kind::Symlink,
			Value::Placeholder(_) => Kind::Placeholder,
			Value::Template(_) => Kind::Template,
			Value::Package(_) => Kind::Package,
			Value::Resource(_) => Kind::Resource,
			Value::Target(_) => Kind::Target,
			Value::Task(_) => Kind::Task,
			Value::Array(_) => Kind::Array,
			Value::Object(_) => Kind::Object,
		}
	}

	pub(crate) fn expect_id(&self) -> Id {
		self.id.read().unwrap().unwrap()
	}

	pub async fn id(&self, client: &Client) -> Result<Id> {
		// Store the value.
		self.store(client).await?;

		// Return the ID.
		Ok(self.id.read().unwrap().unwrap())
	}

	pub async fn value(&self, client: &Client) -> Result<&Value> {
		// Load the value.
		self.load(client).await?;

		// Return a reference to the value.
		Ok(unsafe { &*(self.value.read().unwrap().as_ref().unwrap() as *const Value) })
	}

	#[allow(clippy::unused_async)]
	pub async fn load(&self, client: &Client) -> Result<()> {
		// If the value is already loaded, then return.
		if self.value.read().unwrap().is_some() {
			return Ok(());
		}

		// Get the id.
		let id = self.id.read().unwrap().unwrap();

		// Get the data.
		let Some(data) = client.try_get_value_bytes(id).await? else {
			return_error!(r#"Failed to find value with id "{id}"."#);
		};

		// Create the value.
		let data = value::Data::deserialize(&data)?;
		let value = Value::from_data(data);

		// Set the value.
		self.value.write().unwrap().replace(value);

		Ok(())
	}

	#[async_recursion]
	pub async fn store(&self, client: &Client) -> Result<()> {
		// If the value is already stored, then return.
		if self.id.read().unwrap().is_some() {
			return Ok(());
		}

		// Store the children.
		let children = self.value.read().unwrap().as_ref().unwrap().children();
		children
			.into_iter()
			.map(|child| async move { child.store(client).await })
			.collect::<FuturesUnordered<_>>()
			.try_collect()
			.await?;

		// Serialize the data.
		let data = self.value.read().unwrap().as_ref().unwrap().to_data();
		let data = data.serialize()?;
		let id = Id::new(self.kind(), &data);

		// Store the value.
		client
			.try_put_value(id, &data)
			.await
			.wrap_err("Failed to put the value.")?
			.map_err(|_| error!("Expected all children to be stored."))?;

		// Set the ID.
		self.id.write().unwrap().replace(id);

		Ok(())
	}

	pub async fn store_direct(&self, server: &Server) -> Result<()> {
		// If the handle is already stored, then return.
		if self.id.read().unwrap().is_some() {
			return Ok(());
		}

		let handle = self.clone();
		let server = server.clone();
		tokio::task::spawn_blocking(move || {
			// Begin a write transaction.
			let mut txn = server.state.database.env.begin_rw_txn()?;

			// Collect the stored handles.
			let mut stored = Vec::new();

			// Store the handle and its unstored children recursively.
			handle.store_direct_inner(&server, &mut txn, &mut stored)?;

			// Commit the transaction.
			txn.commit()?;

			// Set the IDs of the stored handles.
			for (id, handle) in stored {
				handle.id.write().unwrap().replace(id);
			}

			Ok::<_, Error>(())
		})
		.await
		.map_err(Error::other)
		.wrap_err("Failed to join the store task.")?
		.wrap_err("Failed to store the value.")?;
		Ok(())
	}

	fn store_direct_inner(
		&self,
		server: &Server,
		txn: &mut lmdb::RwTransaction,
		stored: &mut Vec<(Id, Handle)>,
	) -> Result<()> {
		// If the handle is already stored, then return.
		if self.id.read().unwrap().is_some() {
			return Ok(());
		}

		// Otherwise, it must be loaded, so get the value.
		let value = self.value.read().unwrap();
		let value = value.as_ref().unwrap();

		// Store the children.
		for child in value.children() {
			child.store_direct_inner(server, txn, stored)?;
		}

		// Serialize the data.
		let data = value.to_data();
		let bytes = data.serialize()?;
		let id = Id::new(self.kind(), &bytes);

		// Add the value to the database.
		txn.put(
			server.state.database.values,
			&id.as_bytes(),
			&bytes,
			lmdb::WriteFlags::empty(),
		)?;

		// Add to the stored handles.
		stored.push((id, self.clone()));

		Ok(())
	}
}
