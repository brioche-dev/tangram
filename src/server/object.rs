use super::Server;
use crate::{
	hash::Hash,
	object::{BlobHash, Object, ObjectHash},
	util::path_exists,
};
use anyhow::Result;
use std::sync::Arc;

pub enum AddObjectOutcome {
	Added(ObjectHash),
	DirectoryMissingEntries(Vec<(String, ObjectHash)>),
	FileMissingBlob(BlobHash),
	DependencyMissing(ObjectHash),
}

impl Server {
	// Add an object to the server after ensuring the server has all its references.
	pub async fn add_object(self: &Arc<Self>, object: &Object) -> Result<AddObjectOutcome> {
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
					return Ok(AddObjectOutcome::DirectoryMissingEntries(missing_entries));
				}
			},

			// If this object is a file, ensure its blob is present.
			Object::File(file) => {
				let blob_path = self.blob_path(file.blob_hash);
				let blob_exists = path_exists(&blob_path).await?;
				if !blob_exists {
					return Ok(AddObjectOutcome::FileMissingBlob(file.blob_hash));
				}
			},

			// If this object is a symlink, there is nothing to ensure.
			Object::Symlink(_) => {},

			// If this object is a dependency, ensure it is present.
			Object::Dependency(dependency) => {
				let object_hash = dependency.artifact.object_hash();
				if !self.object_exists(object_hash).await? {
					return Ok(AddObjectOutcome::DependencyMissing(object_hash));
				}
			},
		}

		// Serialize the object.
		let object_data = serde_json::to_vec(&object)?;

		// Hash the object.
		let object_hash = ObjectHash(Hash::new(&object_data));

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

		Ok(AddObjectOutcome::Added(object_hash))
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
