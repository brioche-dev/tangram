use super::Server;
use crate::{
	artifact::Artifact,
	hash::BuildHasher,
	object::{BlobHash, Object, ObjectHash},
	util::rmrf,
};
use anyhow::{Context, Result};
use rusqlite::Transaction;
use std::collections::HashSet;
use std::sync::Arc;

impl Server {
	pub async fn garbage_collect(self: &Arc<Self>) -> Result<()> {
		// Acquire a write guard to the garbage collection lock.
		let gc_lock = self.gc_lock.write().await;

		// Retrieve a database connection.
		let database_connection_object = self
			.database_connection_pool
			.get()
			.await
			.context("Failed to retrieve a database connection.")?;

		let marked_blob_hashes =
			tokio::task::block_in_place(move || -> Result<HashSet<BlobHash, BuildHasher>> {
				// Create a database transaction.
				let mut database_connection = database_connection_object.lock().unwrap();
				let txn = database_connection.transaction()?;

				// Get the gc roots.
				let roots = get_roots(&txn)?;

				// Traverse the transitive dependencies of the roots and add each hash to marked_object_hashes and marked_blob_hashes.
				let mut queue: Vec<Object> = Vec::new();
				for root in roots {
					let object_hash = root.object_hash();
					let object = get_object(&txn, object_hash)?;
					queue.push(object);
				}
				let mut marked_object_hashes: HashSet<ObjectHash, BuildHasher> = HashSet::default();
				let mut marked_blob_hashes: HashSet<BlobHash, BuildHasher> = HashSet::default();
				while let Some(object) = queue.pop() {
					let object_hash = object.hash();
					match object {
						Object::File(file) => {
							// Add the object hash to the marked_object_hashes.
							marked_object_hashes.insert(object_hash);
							// Add blob hash to marked_blob_hashes.
							marked_blob_hashes.insert(file.blob_hash);
						},

						Object::Directory(dir) => {
							// Add children to the queue
							for (_, entry) in dir.entries {
								// Add the object hash to the marked_object_hashes.
								marked_object_hashes.insert(entry);

								// Get the object corresponding to this entry.
								let object = get_object(&txn, entry)?;
								queue.push(object);
							}
						},

						Object::Symlink(_) => {
							continue;
						},

						Object::Dependency(dependency) => {
							// Get the object corresponding to this dependency.
							let object = get_object(&txn, dependency.artifact.object_hash())?;
							queue.push(object);
						},
					}
				}

				// Get all of the objects.
				let object_hashes = get_objects(&txn)?;

				// Go through each object and delete it.
				for object_hash in object_hashes {
					if !marked_object_hashes.contains(&object_hash) {
						// Delete the object.
						delete_object(&txn, object_hash)?;
					}
				}

				Ok(marked_blob_hashes)
			})?;

		// Go through the blobs dir, if the blob hash is not in the marked blob hash set, delete it.
		let mut read_dir = tokio::fs::read_dir(self.blobs_dir())
			.await
			.context("Failed to read the directory.")?;
		while let Some(entry) = read_dir.next_entry().await? {
			let blob_hash: BlobHash = entry.file_name().to_str().unwrap().parse()?;
			if !marked_blob_hashes.contains(&blob_hash) {
				rmrf(&entry.path(), None)
					.await
					.with_context(|| "Failed to remove the blob.")?;
			}
		}

		// Delete all temps.
		rmrf(&self.temps_dir(), None).await?;

		// Drop the write guard on the garbage collection lock.
		drop(gc_lock);

		Ok(())
	}
}

fn get_roots(txn: &Transaction) -> Result<Vec<Artifact>> {
	let sql = r#"
		select
			artifact_hash
		from roots
	"#;
	let mut statement = txn
		.prepare_cached(sql)
		.context("Failed to prepare the query.")?;
	let roots = statement
		.query_map((), |row| {
			let hash = row.get::<_, String>(0);
			hash.map(|hash| {
				let hash = hash
					.parse()
					.with_context(|| "Failed to parse object hash.")
					.unwrap();
				Artifact::new(hash)
			})
		})
		.context("Failed to execute the query.")?
		.collect::<rusqlite::Result<Vec<Artifact>>>()?;
	drop(statement);
	Ok(roots)
}

fn get_object(txn: &Transaction, object_hash: ObjectHash) -> Result<Object> {
	let sql = r#"
		select
			data
		from objects
		where
			hash = $1
	"#;
	let mut statement = txn
		.prepare_cached(sql)
		.context("Failed to prepare the query.")?;
	let object_data = statement
		.query_row((object_hash.to_string(),), |row| row.get::<_, Vec<u8>>(0))
		.context("Failed to execute the query.")?;

	let object = serde_json::from_slice(&object_data)?;

	Ok(object)
}

fn get_objects(txn: &Transaction) -> Result<Vec<ObjectHash>> {
	let sql = r#"
		select object_hash from objects
	"#;

	let mut statement = txn
		.prepare_cached(sql)
		.context("Failed to prepare the query.")?;
	let object_hashes = statement
		.query_map((), |row| {
			row.get::<_, String>(0).map(|hash| {
				hash.parse()
					.with_context(|| "Failed to parse object hash.")
					.unwrap()
			})
		})
		.context("Failed to execute the query.")?
		.collect::<rusqlite::Result<Vec<ObjectHash>>>()?;

	Ok(object_hashes)
}

fn delete_object(txn: &Transaction, object_hash: ObjectHash) -> Result<()> {
	let sql = r#"
		delete from
			objects
		where
			hash = $1
	"#;
	let mut statement = txn
		.prepare_cached(sql)
		.context("Failed to prepare the query.")?;
	statement
		.execute((object_hash.to_string(),))
		.context("Failed to execute the query.")?;
	Ok(())
}
