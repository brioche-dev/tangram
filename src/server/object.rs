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
					if !self.object_exists(*object_hash).await? {
						missing_entries.push((entry_name.clone(), *object_hash));
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
				if !self.object_exists(object_hash).await? {
					return Ok(AddObjectOutcome::DependencyMissing { object_hash });
				}
			},
		}

		// Serialize the object.
		let object_data = serde_json::to_vec(&object)?;

		// Add the object to the database.
		self.database_execute(
			r#"
				replace into objects (
					hash, data
				) values (
					$1, $2
				)
			"#,
			(object_hash.to_string(), object_data),
		)
		.await?;

		Ok(AddObjectOutcome::Added { object_hash })
	}

	pub async fn object_exists(self: &Arc<Self>, object_hash: ObjectHash) -> Result<bool> {
		let exists = self
			.database_query_row(
				r#"
					select count(*) > 0 from objects where hash = $1
				"#,
				(object_hash.to_string(),),
				|row| Ok(row.get::<_, bool>(0)?),
			)
			.await?
			.unwrap();
		Ok(exists)
	}

	pub async fn get_object(self: &Arc<Self>, object_hash: ObjectHash) -> Result<Option<Object>> {
		let object_data = self
			.database_query_row(
				"
					select
						data
					from objects
					where hash = $1;
				",
				(object_hash.to_string(),),
				|row| Ok(row.get::<_, Vec<u8>>(0)?),
			)
			.await?;
		let object = if let Some(object_data) = object_data {
			let object = serde_json::from_slice(&object_data)?;
			Some(object)
		} else {
			None
		};
		Ok(object)
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
		let outcome = self.add_object(object_hash, &object).await?;

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
