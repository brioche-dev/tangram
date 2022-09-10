use super::Server;
use crate::{
	artifact::Artifact,
	blob, hash,
	object::{self, Object},
};
use anyhow::{anyhow, Context, Result};
use std::{
	collections::{HashSet, VecDeque},
	path::Path,
	sync::Arc,
};

impl Server {
	pub async fn garbage_collect(self: &Arc<Self>) -> Result<()> {
		// Acquire a write guard to the garbage collection lock.
		let gc_lock = self.gc_lock.write().await;

		// Create hash sets to track the marked objects and blobs.
		let mut marked_object_hashes: HashSet<object::Hash, hash::BuildHasher> = HashSet::default();
		let mut marked_blob_hashes: HashSet<blob::Hash, hash::BuildHasher> = HashSet::default();

		self.database_transaction(|txn| {
			// Mark the objects and blobs.
			Self::mark(txn, &mut marked_object_hashes, &mut marked_blob_hashes)
				.context("Failed to mark the objects and blobs.")?;

			// Sweep the objects.
			Self::sweep_objects(txn, &marked_object_hashes)
				.context("Failed to sweep the objects.")?;

			Ok(())
		})
		.await?;

		// Sweep the blobs.
		self.sweep_blobs(&self.blobs_path(), &marked_blob_hashes)
			.await
			.context("Failed to sweep the blobs.")?;

		// Delete all temps.
		tokio::fs::remove_dir_all(&self.temps_path())
			.await
			.context("Failed to delete the temps directory.")?;
		tokio::fs::create_dir_all(&self.temps_path())
			.await
			.context("Failed to recreate the temps directory.")?;

		// Drop the write guard to the garbage collection lock.
		drop(gc_lock);

		Ok(())
	}

	fn mark(
		txn: &rusqlite::Transaction,
		marked_object_hashes: &mut HashSet<object::Hash, hash::BuildHasher>,
		marked_blob_hashes: &mut HashSet<blob::Hash, hash::BuildHasher>,
	) -> Result<()> {
		// Get the roots.
		let sql = r#"
			select artifact_hash from roots
		"#;
		let mut statement = txn
			.prepare_cached(sql)
			.context("Failed to prepare the query.")?;
		let roots = statement
			.query(())
			.context("Failed to execute the query.")?
			.and_then(|row| {
				let hash = row.get::<_, String>(0)?;
				let object_hash = hash
					.parse()
					.with_context(|| "Failed to parse the object hash.")?;
				let artifact = Artifact::new(object_hash);
				Ok::<_, anyhow::Error>(artifact)
			});

		// Traverse the transitive dependencies of the roots and add each hash to the marked object hashes and marked blob hashes.
		let mut queue: VecDeque<Object> = VecDeque::new();
		for root in roots {
			let root = root?;
			let object_hash = root.object_hash();
			let object = Self::get_object_with_transaction(txn, object_hash)?
				.ok_or_else(|| anyhow!(r#"Failed to find object with hash "{}"."#, object_hash))?;
			queue.push_back(object);
		}
		while let Some(object) = queue.pop_front() {
			let object_hash = object.hash();

			// Mark this object.
			marked_object_hashes.insert(object_hash);

			match object {
				// If the object is a file, mark its blob.
				Object::File(file) => {
					marked_blob_hashes.insert(file.blob_hash);
				},

				// If the object is a directory, add its entries to the queue.
				Object::Directory(directory) => {
					for (_, object_hash) in directory.entries {
						let object = Self::get_object_with_transaction(txn, object_hash)?
							.ok_or_else(|| {
								anyhow!(r#"Failed to find object with hash "{}"."#, object_hash)
							})?;
						queue.push_back(object);
					}
				},

				// There is nothing to do for a symlink.
				Object::Symlink(_) => {
					continue;
				},

				// If the object is a dependency, add the depended upon object to the queue.
				Object::Dependency(dependency) => {
					let object_hash = dependency.artifact.object_hash();
					let object =
						Self::get_object_with_transaction(txn, object_hash)?.ok_or_else(|| {
							anyhow!(r#"Failed to find object with hash "{}"."#, object_hash)
						})?;
					queue.push_back(object);
				},
			}
		}

		Ok(())
	}

	fn sweep_objects(
		txn: &rusqlite::Transaction,
		marked_object_hashes: &HashSet<object::Hash, hash::BuildHasher>,
	) -> Result<()> {
		// Get all of the objects.
		let sql = r#"
			select object_hash from objects
		"#;
		let mut statement = txn
			.prepare_cached(sql)
			.context("Failed to prepare the query.")?;
		let object_hashes = statement
			.query(())
			.context("Failed to execute the query.")?
			.and_then(|row| {
				let hash = row.get::<_, String>(0)?;
				let hash = hash
					.parse()
					.with_context(|| "Failed to parse object hash.")?;
				Ok::<_, anyhow::Error>(hash)
			});

		// Go through each object and delete it.
		for object_hash in object_hashes {
			let object_hash = object_hash?;
			if !marked_object_hashes.contains(&object_hash) {
				Self::delete_object_with_transaction(txn, object_hash)?;
			}
		}

		Ok(())
	}

	async fn sweep_blobs(
		self: &Arc<Self>,
		blobs_path: &Path,
		marked_blob_hashes: &HashSet<blob::Hash, hash::BuildHasher>,
	) -> Result<()> {
		// Read the files in the blobs directory and delete each file that is not marked.
		let mut read_dir = tokio::fs::read_dir(blobs_path)
			.await
			.context("Failed to read the directory.")?;
		while let Some(entry) = read_dir.next_entry().await? {
			let blob_hash: blob::Hash = entry
				.file_name()
				.to_str()
				.context("Failed to parse the file name as a string.")?
				.parse()
				.context("Failed to parse the entry in the blobs directory as a blob hash.")?;
			if !marked_blob_hashes.contains(&blob_hash) {
				tokio::fs::remove_file(&entry.path())
					.await
					.context("Failed to remove the blob.")?;
			}
		}
		Ok(())
	}
}
