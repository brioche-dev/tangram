use super::Server;
use bytes::Bytes;
use futures::{stream, StreamExt, TryStreamExt};
use lmdb::Transaction;
use tangram_client as tg;
use tangram_util::{bad_request, empty, full, not_found, Incoming, Outgoing};
use tg::{object, return_error, Error, Result, Wrap, WrapErr};
use tokio::io::AsyncReadExt;

impl Server {
	pub async fn handle_head_object_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "objects", id] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let Ok(id) = id.parse() else {
			return Ok(bad_request());
		};

		let exists = self.get_object_exists(&id).await?;

		// Create the response.
		let status = if exists {
			http::StatusCode::OK
		} else {
			http::StatusCode::NOT_FOUND
		};
		let response = http::Response::builder()
			.status(status)
			.body(empty())
			.unwrap();

		Ok(response)
	}

	pub async fn handle_get_object_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "objects", id] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let Ok(id) = id.parse() else {
			return Ok(bad_request());
		};

		let Some(bytes) = self.try_get_object_bytes(&id).await? else {
			return Ok(not_found());
		};

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(bytes))
			.unwrap();

		Ok(response)
	}

	pub async fn handle_put_object_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "objects", id] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let Ok(id) = id.parse() else {
			return Ok(bad_request());
		};

		// Create a reader from the body.
		let mut body = tokio_util::io::StreamReader::new(
			http_body_util::BodyStream::new(request.into_body())
				.try_filter_map(|frame| Box::pin(async move { Ok(frame.into_data().ok()) }))
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error)),
		);

		// Read the body.
		let mut bytes = Vec::new();
		body.read_to_end(&mut bytes)
			.await
			.wrap_err("Failed to read the body.")?;
		let bytes = bytes.into();

		// Put the object.
		let result = self.try_put_object_bytes(&id, &bytes).await?;

		// If there are missing children, then return a bad request response.
		if let Err(missing_children) = result {
			let body = serde_json::to_vec(&missing_children)
				.wrap_err("Failed to serialize the missing children.")?;
			let response = http::Response::builder()
				.status(http::StatusCode::BAD_REQUEST)
				.body(full(body))
				.unwrap();
			return Ok(response);
		}

		// Otherwise, return an ok response.
		Ok(http::Response::builder()
			.status(http::StatusCode::OK)
			.body(empty())
			.unwrap())
	}

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
			.state
			.database
			.env
			.begin_ro_txn()
			.wrap_err("Failed to create the transaction.")?;
		match txn.get(self.state.database.objects, &id.to_bytes()) {
			Ok(_) => Ok(true),
			Err(lmdb::Error::NotFound) => Ok(false),
			Err(error) => Err(error.wrap("Failed to get the object.")),
		}
	}

	async fn get_object_exists_from_parent(&self, id: &object::Id) -> Result<bool> {
		if let Some(parent) = self.state.parent.as_ref() {
			if parent.get_object_exists(id).await? {
				return Ok(true);
			}
		}
		Ok(false)
	}

	pub async fn get_object_bytes(&self, id: &object::Id) -> Result<Bytes> {
		self.try_get_object_bytes(id)
			.await?
			.wrap_err("Failed to get the object.")
	}

	pub async fn try_get_object_bytes(&self, id: &object::Id) -> Result<Option<Bytes>> {
		// Attempt to get the object from the database.
		if let Some(bytes) = self.try_get_object_bytes_from_database(id)? {
			return Ok(Some(bytes));
		};

		// Attempt to get the object from the parent.
		if let Ok(Some(bytes)) = self.try_get_object_bytes_from_parent(id).await {
			return Ok(Some(bytes));
		};

		Ok(None)
	}

	pub fn try_get_object_bytes_from_database(&self, id: &object::Id) -> Result<Option<Bytes>> {
		let txn = self
			.state
			.database
			.env
			.begin_ro_txn()
			.wrap_err("Failed to create the transaction.")?;
		match txn.get(self.state.database.objects, &id.to_bytes()) {
			Ok(data) => Ok(Some(Bytes::copy_from_slice(data))),
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => Err(error.wrap("Failed to get the object.")),
		}
	}

	async fn try_get_object_bytes_from_parent(&self, id: &object::Id) -> Result<Option<Bytes>> {
		let Some(parent) = self.state.parent.as_ref() else {
			return Ok(None);
		};

		// Get the object from the parent.
		let Some(bytes) = parent.try_get_object_bytes(id).await? else {
			return Ok(None);
		};

		// Create a write transaction.
		let mut txn = self
			.state
			.database
			.env
			.begin_rw_txn()
			.wrap_err("Failed to create the transaction.")?;

		// Add the object to the database.
		txn.put(
			self.state.database.objects,
			&id.to_bytes(),
			&bytes,
			lmdb::WriteFlags::empty(),
		)
		.wrap_err("Failed to put the object.")?;

		// Commit the transaction.
		txn.commit().wrap_err("Failed to commit the transaction.")?;

		Ok(Some(bytes))
	}

	/// Attempt to put a object.
	pub async fn try_put_object_bytes(
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
			.state
			.database
			.env
			.begin_rw_txn()
			.wrap_err("Failed to create the transaction.")?;

		// Add the object to the database.
		txn.put(
			self.state.database.objects,
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
