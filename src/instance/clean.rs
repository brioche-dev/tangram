use super::Instance;
use crate::{
	artifact::{self, Artifact},
	blob::{self, Blob},
	error::{Error, Result, WrapErr},
	hash,
	operation::{self, Operation},
	template,
	value::Value,
};
use lmdb::{Cursor, Transaction};
use std::collections::{HashSet, VecDeque};

impl Instance {
	pub async fn clean(&self, roots: Vec<Operation>) -> Result<()> {
		// Create marks to track marked artifacts, blobs, operations.
		let mut marks = Marks::default();

		// Mark the artifacts, blobs, operations.
		self.mark(&mut marks, roots)
			.await
			.wrap_err("Failed to perform the mark phase.")?;

		// Sweep the artifacts.
		self.sweep_artifacts(&marks)
			.wrap_err("Failed to sweep the artifacts.")?;

		// Sweep the artifacts directory.
		self.sweep_artifacts_directory(&marks)
			.await
			.wrap_err("Failed to sweep the artifacts directory.")?;

		// Sweep the blobs.
		self.sweep_blobs(&marks)
			.await
			.wrap_err("Failed to sweep the blobs.")?;

		// Sweep the operations.
		self.sweep_operations(&marks)
			.wrap_err("Failed to sweep the operations.")?;

		// Sweep the operation children.
		self.sweep_operation_children(&marks)
			.wrap_err("Failed to sweep the operation children.")?;

		// Sweep the operation outputs.
		self.sweep_operation_outputs(&marks)
			.wrap_err("Failed to sweep the operation outputs.")?;

		// Delete all temps.
		tokio::fs::remove_dir_all(&self.temps_path())
			.await
			.wrap_err("Failed to delete the temps directory.")?;
		tokio::fs::create_dir_all(&self.temps_path())
			.await
			.wrap_err("Failed to recreate the temps directory.")?;

		Ok(())
	}
}

enum QueueItem {
	Artifact(Artifact),
	Blob(Blob),
	Operation(Operation),
	Value(Value),
}

impl Instance {
	#[allow(clippy::too_many_lines)]
	async fn mark(&self, marks: &mut Marks, roots: Vec<Operation>) -> Result<()> {
		let mut queue: VecDeque<QueueItem> = roots.into_iter().map(QueueItem::Operation).collect();
		while let Some(item) = queue.pop_front() {
			match item {
				QueueItem::Artifact(artifact) => {
					// Mark the artifact.
					marks.mark_artifact(artifact.hash());

					match artifact {
						Artifact::Directory(directory) => {
							// Add the entries to the queue.
							for artifact in directory.entries(self).await?.into_values() {
								queue.push_back(QueueItem::Artifact(artifact));
							}
						},

						Artifact::File(file) => {
							// Mark the blob.
							marks.mark_blob(file.blob().hash());

							for artifact in file.references(self).await? {
								queue.push_back(QueueItem::Artifact(artifact));
							}
						},

						Artifact::Symlink(symlink) => {
							queue.push_back(QueueItem::Value(Value::Template(
								symlink.target().clone(),
							)));
						},
					}
				},

				QueueItem::Blob(blob) => {
					// Mark the blob.
					marks.mark_blob(blob.hash());
				},

				QueueItem::Operation(operation) => {
					// Mark the operation.
					marks.mark_operation(operation.hash());

					// Add this operations's children to the queue.
					let children = operation.children(self).await?;
					for operation in children {
						queue.push_back(QueueItem::Operation(operation));
					}

					// Get the output and add it to the queue.
					let output = operation.try_get_output(self).await?;
					if let Some(value) = output {
						queue.push_back(QueueItem::Value(value));
					}

					match operation {
						Operation::Resource(_) => {},

						Operation::Command(command) => {
							// Add the executable to the queue.
							queue.push_back(QueueItem::Value(Value::Template(
								command.executable().clone(),
							)));

							// Add the env to the queue.
							for template in command.env().values() {
								queue
									.push_back(QueueItem::Value(Value::Template(template.clone())));
							}

							// Add the args to the queue.
							for template in command.args() {
								queue
									.push_back(QueueItem::Value(Value::Template(template.clone())));
							}
						},

						Operation::Function(function) => {
							// Add the package to the queue.
							queue.push_back(QueueItem::Artifact(
								function.package(self).await?.artifact().clone(),
							));

							// Add the env to the queue.
							for value in function.env.into_values() {
								queue.push_back(QueueItem::Value(value));
							}

							// Add the args to the queue.
							for value in function.args {
								queue.push_back(QueueItem::Value(value));
							}
						},
					}
				},

				QueueItem::Value(value) => match value {
					Value::Null
					| Value::Bool(_)
					| Value::Number(_)
					| Value::String(_)
					| Value::Bytes(_)
					| Value::Subpath(_)
					| Value::Relpath(_)
					| Value::Placeholder(_) => {},

					Value::Blob(blob) => {
						// Add the blob to the queue.
						queue.push_back(QueueItem::Blob(blob));
					},

					Value::Artifact(artifact) => {
						// Add the artifact to the queue.
						queue.push_back(QueueItem::Artifact(artifact));
					},

					Value::Template(template) => {
						for component in template.components() {
							match component {
								template::Component::String(_)
								| template::Component::Placeholder(_) => {},
								template::Component::Artifact(artifact) => {
									queue.push_back(QueueItem::Artifact(artifact.clone()));
								},
							}
						}
					},

					Value::Operation(operation) => {
						// Add the artifact to the queue.
						queue.push_back(QueueItem::Operation(operation));
					},

					Value::Array(array) => {
						// Add the values to the queue.
						for value in array {
							queue.push_back(QueueItem::Value(value));
						}
					},

					Value::Object(object) => {
						// Add the values to the queue.
						for value in object.into_values() {
							queue.push_back(QueueItem::Value(value));
						}
					},
				},
			}
		}

		Ok(())
	}
}

impl Instance {
	fn sweep_artifacts(&self, marks: &Marks) -> Result<()> {
		// Open a read/write transaction.
		let mut txn = self.database.env.begin_rw_txn()?;

		// Open a read/write cursor.
		let mut cursor = txn.open_rw_cursor(self.database.artifacts)?;

		// Delete all artifacts that are not marked.
		for entry in cursor.iter() {
			let (hash, _) = entry?;
			let hash = hash.try_into().map_err(Error::other)?;
			let hash = artifact::Hash(hash::Hash(hash));
			if !marks.contains_artifact(hash) {
				cursor.del(lmdb::WriteFlags::empty())?;
			}
		}

		// Drop the cursor.
		drop(cursor);

		// Commit the transaction.
		txn.commit()?;

		Ok(())
	}

	async fn sweep_artifacts_directory(&self, marks: &Marks) -> Result<()> {
		// Delete all entries in the artifacts directory that are not marked.
		let mut read_dir = tokio::fs::read_dir(self.artifacts_path())
			.await
			.wrap_err("Failed to read the artifacts directory.")?;
		while let Some(entry) = read_dir.next_entry().await? {
			let artifact_hash: hash::Hash = entry
				.file_name()
				.to_str()
				.wrap_err("Failed to parse the file name as a string.")?
				.parse()
				.map_err(Error::other)
				.wrap_err("Failed to parse the entry in the artifacts directory as a hash.")?;
			let artifact_hash = artifact::Hash(artifact_hash);
			if !marks.contains_artifact(artifact_hash) {
				crate::util::fs::rmrf(&entry.path())
					.await
					.wrap_err("Failed to remove the artifact.")?;
			}
		}
		Ok(())
	}

	async fn sweep_blobs(&self, marks: &Marks) -> Result<()> {
		// Delete all blobs that are not not marked.
		let mut read_dir = tokio::fs::read_dir(self.blobs_path())
			.await
			.wrap_err("Failed to read the blobs directory.")?;
		while let Some(entry) = read_dir.next_entry().await? {
			let blob_hash: hash::Hash = entry
				.file_name()
				.to_str()
				.wrap_err("Failed to parse the file name as a string.")?
				.parse()
				.map_err(Error::other)
				.wrap_err("Failed to parse the entry in the blobs directory as a hash.")?;
			let blob_hash = blob::Hash(blob_hash);
			if !marks.contains_blob(blob_hash) {
				tokio::fs::remove_file(&entry.path())
					.await
					.wrap_err("Failed to remove the blob.")?;
			}
		}
		Ok(())
	}

	fn sweep_operations(&self, marks: &Marks) -> Result<()> {
		// Open a read/write transaction.
		let mut txn = self.database.env.begin_rw_txn()?;

		// Open a read/write cursor.
		let mut cursor = txn.open_rw_cursor(self.database.operations)?;

		// Delete all operations that are not marked.
		for entry in cursor.iter() {
			let (hash, _) = entry?;
			let hash = hash.try_into().map_err(Error::other)?;
			let hash = operation::Hash(hash::Hash(hash));
			if !marks.contains_operation(hash) {
				cursor.del(lmdb::WriteFlags::empty())?;
			}
		}

		// Drop the cursor.
		drop(cursor);

		// Commit the transaction.
		txn.commit()?;

		Ok(())
	}

	fn sweep_operation_children(&self, marks: &Marks) -> Result<()> {
		// Open a read/write transaction.
		let mut txn = self.database.env.begin_rw_txn()?;

		// Open a read/write cursor.
		let mut cursor = txn.open_rw_cursor(self.database.operation_children)?;

		// Delete the children of all operations that are not marked.
		for entry in cursor.iter() {
			let (hash, _) = entry?;
			let hash = hash.try_into().map_err(Error::other)?;
			let hash = operation::Hash(hash::Hash(hash));
			if !marks.contains_operation(hash) {
				cursor.del(lmdb::WriteFlags::empty())?;
			}
		}

		// Drop the cursor.
		drop(cursor);

		// Commit the transaction.
		txn.commit()?;

		Ok(())
	}

	fn sweep_operation_outputs(&self, marks: &Marks) -> Result<()> {
		// Open a read/write transaction.
		let mut txn = self.database.env.begin_rw_txn()?;

		// Open a read/write cursor.
		let mut cursor = txn.open_rw_cursor(self.database.operation_outputs)?;

		// Delete the outputs of all operations that are not marked.
		for entry in cursor.iter() {
			let (hash, _) = entry?;
			let hash = hash.try_into().map_err(Error::other)?;
			let hash = operation::Hash(hash::Hash(hash));
			if !marks.contains_operation(hash) {
				cursor.del(lmdb::WriteFlags::empty())?;
			}
		}

		// Drop the cursor.
		drop(cursor);

		// Commit the transaction.
		txn.commit()?;

		Ok(())
	}
}

#[derive(Default)]
struct Marks {
	artifacts: HashSet<artifact::Hash, hash::BuildHasher>,
	blobs: HashSet<blob::Hash, hash::BuildHasher>,
	operations: HashSet<operation::Hash, hash::BuildHasher>,
}

impl Marks {
	fn mark_artifact(&mut self, artifact_hash: artifact::Hash) {
		self.artifacts.insert(artifact_hash);
	}

	fn contains_artifact(&self, artifact_hash: artifact::Hash) -> bool {
		self.artifacts.contains(&artifact_hash)
	}

	fn mark_blob(&mut self, blob_hash: blob::Hash) {
		self.blobs.insert(blob_hash);
	}

	fn contains_blob(&self, blob_hash: blob::Hash) -> bool {
		self.blobs.contains(&blob_hash)
	}

	fn mark_operation(&mut self, operation_hash: operation::Hash) {
		self.operations.insert(operation_hash);
	}

	fn contains_operation(&self, operation_hash: operation::Hash) -> bool {
		self.operations.contains(&operation_hash)
	}
}
