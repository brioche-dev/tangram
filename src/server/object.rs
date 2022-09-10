use super::{error::bad_request, Server};
use crate::{
	object::{BlobHash, Object, ObjectHash},
	util::path_exists,
};
use anyhow::{bail, Context, Result};
use std::sync::Arc;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AddObjectOutcome {
	Added { object_hash: ObjectHash },
	DirectoryMissingEntries { entries: Vec<(String, ObjectHash)> },
	FileMissingBlob { blob_hash: BlobHash },
	DependencyMissing { object_hash: ObjectHash },
}

impl Server {
	// Add an object to the server after ensuring the server has all its references.
	pub async fn add_object(
		self: &Arc<Self>,
		object_hash: ObjectHash,
		object: &Object,
	) -> Result<AddObjectOutcome> {
		// Before adding this object, we need to ensure the server has all its references.
		match &object {
			// If this object is a directory, ensure all its entries are present.
			Object::Directory(directory) => {
				let mut missing_entries = Vec::new();
				for (entry_name, object_hash) in &directory.entries {
					let object_hash = *object_hash;
					let object_exists = self
						.database_transaction(|txn| {
							let object_exists =
								Self::object_exists_with_transaction(txn, object_hash)?;
							Ok(object_exists)
						})
						.await?;
					if !object_exists {
						missing_entries.push((entry_name.clone(), object_hash));
					}
				}
				if !missing_entries.is_empty() {
					return Ok(AddObjectOutcome::DirectoryMissingEntries {
						entries: missing_entries,
					});
				}
			},

			// If this object is a file, ensure its blob is present.
			Object::File(file) => {
				let blob_path = self.blob_path(file.blob_hash);
				let blob_exists = path_exists(&blob_path).await?;
				if !blob_exists {
					return Ok(AddObjectOutcome::FileMissingBlob {
						blob_hash: file.blob_hash,
					});
				}
			},

			// If this object is a symlink, there is nothing to ensure.
			Object::Symlink(_) => {},

			// If this object is a dependency, ensure it is present.
			Object::Dependency(dependency) => {
				let object_hash = dependency.artifact.object_hash();
				let object_exists = self
					.database_transaction(|txn| {
						let object_exists = Self::object_exists_with_transaction(txn, object_hash)?;
						Ok(object_exists)
					})
					.await?;
				if !object_exists {
					return Ok(AddObjectOutcome::DependencyMissing { object_hash });
				}
			},
		}

		// Serialize the object.
		let object_data = serde_json::to_vec(&object)?;

		// Add the object to the database.
		self.database_transaction(|txn| {
			txn.execute(
				r#"
					replace into objects (
						hash, data
					) values (
						$1, $2
					)
				"#,
				(object_hash.to_string(), object_data),
			)?;
			Ok(())
		})
		.await?;

		Ok(AddObjectOutcome::Added { object_hash })
	}

	pub fn object_exists_with_transaction(
		txn: &rusqlite::Transaction<'_>,
		object_hash: ObjectHash,
	) -> Result<bool> {
		let sql = r#"
			select count(*) > 0 from objects where hash = $1
		"#;
		let params = (object_hash.to_string(),);
		let exists = txn
			.prepare_cached(sql)
			.context("Failed to prepare the query.")?
			.query(params)
			.context("Failed to execute the query.")?
			.and_then(|row| row.get::<_, bool>(0))
			.next()
			.unwrap()?;
		Ok(exists)
	}

	pub fn get_object_with_transaction(
		txn: &rusqlite::Transaction,
		object_hash: ObjectHash,
	) -> Result<Option<Object>> {
		let sql = r#"
			select
				data
			from objects
			where
				hash = $1
		"#;
		let params = (object_hash.to_string(),);
		let mut statement = txn
			.prepare_cached(sql)
			.context("Failed to prepare the query.")?;
		let maybe_object = statement
			.query(params)?
			.and_then(|row| {
				let object_data = row.get::<_, Vec<u8>>(0)?;
				let object = serde_json::from_slice(&object_data)?;
				Ok::<_, anyhow::Error>(object)
			})
			.next()
			.transpose()?;
		Ok(maybe_object)
	}

	pub async fn get_object(self: &Arc<Self>, object_hash: ObjectHash) -> Result<Option<Object>> {
		self.database_transaction(|txn| Self::get_object_with_transaction(txn, object_hash))
			.await
	}

	pub fn delete_object_with_transaction(
		txn: &rusqlite::Transaction,
		object_hash: ObjectHash,
	) -> Result<()> {
		let sql = r#"
			delete from objects where hash = $1
		"#;
		let params = (object_hash.to_string(),);
		let mut statement = txn
			.prepare_cached(sql)
			.context("Failed to prepare the query.")?;
		statement
			.execute(params)
			.context("Failed to execute the query.")?;
		Ok(())
	}
}

pub type AddObjectRequest = Object;

pub type AddObjectResponse = AddObjectOutcome;

impl Server {
	pub(super) async fn handle_create_object_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let object_hash = if let ["objects", object_hash] = path_components.as_slice() {
			object_hash
		} else {
			bail!("Unexpected path.");
		};
		let object_hash = match object_hash.parse() {
			Ok(object_hash) => object_hash,
			Err(_) => return Ok(bad_request()),
		};

		// Read and deserialize the request body.
		let body = hyper::body::to_bytes(request.into_body())
			.await
			.context("Failed to read the request body.")?;
		let object =
			serde_json::from_slice(&body).context("Failed to deserialize the request body.")?;

		// Add the object.
		let outcome = self
			.add_object(object_hash, &object)
			.await
			.context("Failed to get the object.")?;

		// Create the response.
		let body =
			serde_json::to_vec(&outcome).context("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}

impl Server {
	pub(super) async fn handle_get_object_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let object_hash = if let ["objects", object_hash] = path_components.as_slice() {
			object_hash
		} else {
			bail!("Unexpected path.");
		};
		let object_hash = match object_hash.parse() {
			Ok(object_hash) => object_hash,
			Err(_) => return Ok(bad_request()),
		};

		// Get the object.
		let object = self
			.database_transaction(|txn| {
				let object = Self::get_object_with_transaction(txn, object_hash)
					.context("Failed to get the object.")?;
				Ok(object)
			})
			.await?;

		// Create the response.
		let body = serde_json::to_vec(&object).context("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}
