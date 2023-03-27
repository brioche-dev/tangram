use crate::{
	artifact::{self, Artifact},
	blob,
	error::{Error, Result, WrapErr},
	hash,
	operation::{self, Operation},
	package, template,
	value::Value,
	Instance,
};
use lmdb::{Cursor, Transaction};
use std::collections::{HashSet, VecDeque};

impl Instance {
	pub async fn clean(&self, roots: Vec<operation::Hash>) -> Result<()> {
		// Create marks to track marked artifacts, blobs, operations, and package instances.
		let mut marks = Marks::default();

		// Mark the artifacts, blobs, operations, and package instances.
		self.mark(&mut marks, roots)
			.wrap_err("Failed to perform the mark phase.")?;

		// Sweep the artifacts.
		self.sweep_artifacts(&marks)
			.wrap_err("Failed to sweep the artifacts.")?;

		// Sweep the checkouts directory.
		self.sweep_checkouts_directory(&marks)
			.await
			.wrap_err("Failed to sweep the checkouts directory.")?;

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

		// Sweep the package instances.
		self.sweep_package_instances(&marks)
			.wrap_err("Failed to sweep the package instances.")?;

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
	Value(Value),
	Artifact(artifact::Hash),
	Operation(operation::Hash),
	PackageInstance(package::instance::Hash),
}

impl Instance {
	#[allow(clippy::too_many_lines)]
	fn mark(&self, marks: &mut Marks, roots: Vec<operation::Hash>) -> Result<()> {
		let txn = self.database.env.begin_ro_txn()?;
		let mut queue: VecDeque<QueueItem> = roots.into_iter().map(QueueItem::Operation).collect();
		while let Some(item) = queue.pop_front() {
			match item {
				QueueItem::Artifact(artifact_hash) => {
					// Mark the artifact.
					marks.mark_artifact(artifact_hash);

					// Get the artifact.
					let artifact = self.get_artifact_local_with_txn(&txn, artifact_hash)?;

					match artifact {
						Artifact::Directory(directory) => {
							// Add the entries to the queue.
							for artifact_hash in directory.entries.into_values() {
								queue.push_back(QueueItem::Artifact(artifact_hash));
							}
						},

						Artifact::File(file) => {
							// Mark the blob.
							marks.mark_blob(file.blob_hash);

							for artifact_hash in &file.references {
								queue.push_back(QueueItem::Artifact(*artifact_hash));
							}
						},

						Artifact::Symlink(symlink) => {
							queue.push_back(QueueItem::Value(Value::Template(symlink.target)));
						},
					}
				},

				QueueItem::Value(value) => match value {
					Value::Null(_)
					| Value::Bool(_)
					| Value::Number(_)
					| Value::String(_)
					| Value::Placeholder(_) => {},

					Value::Artifact(artifact_hash) => {
						// Add the artifact to the queue.
						queue.push_back(QueueItem::Artifact(artifact_hash));
					},

					Value::Template(template) => {
						for component in template.components {
							match component {
								template::Component::String(_)
								| template::Component::Placeholder(_) => {},
								template::Component::Artifact(artifact) => {
									queue.push_back(QueueItem::Artifact(artifact));
								},
							}
						}
					},

					Value::Array(array) => {
						// Add the values to the queue.
						for value in array {
							queue.push_back(QueueItem::Value(value));
						}
					},

					Value::Map(map) => {
						// Add the values to the queue.
						for value in map.into_values() {
							queue.push_back(QueueItem::Value(value));
						}
					},
				},

				QueueItem::Operation(operation_hash) => {
					// Mark the operation.
					marks.mark_operation(operation_hash);

					// Add this operations's children to the queue.
					let children = self.get_operation_children_with_txn(&txn, operation_hash)?;
					for operation_hash in children {
						let operation_hash = operation_hash?;
						queue.push_back(QueueItem::Operation(operation_hash));
					}

					// Get the output and add it to the queue.
					let output = self.get_operation_output(operation_hash)?;
					if let Some(value) = output {
						queue.push_back(QueueItem::Value(value));
					}

					// Get the operation.
					let operation = self.get_operation_local_with_txn(&txn, operation_hash)?;

					match operation {
						Operation::Download(_) => {},

						Operation::Process(process) => {
							// Add the envs to the queue.
							for (_, template) in process.env {
								queue.push_back(QueueItem::Value(Value::Template(template)));
							}

							// Add the command to the queue.
							queue.push_back(QueueItem::Value(Value::Template(process.command)));

							// Add the args to the queue.
							for template in process.args {
								queue.push_back(QueueItem::Value(Value::Template(template)));
							}
						},

						Operation::Call(call) => {
							// Add the package instance to the queue.
							queue.push_back(QueueItem::PackageInstance(
								call.function.package_instance_hash,
							));

							// Add the args to the queue.
							for value in call.args {
								queue.push_back(QueueItem::Value(value));
							}
						},
					}
				},

				QueueItem::PackageInstance(package_instance_hash) => {
					// Mark the package.
					marks.mark_package_instance(package_instance_hash);

					// Get the package.
					let package =
						self.get_package_instance_local_with_txn(&txn, package_instance_hash)?;

					// Add the source to the queue.
					queue.push_back(QueueItem::Artifact(package.package_hash));

					// Mark the package's dependencies.
					for dependency in package.dependencies.into_values() {
						marks.mark_package_instance(dependency);
					}
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

	async fn sweep_checkouts_directory(&self, marks: &Marks) -> Result<()> {
		// Delete all entries in the checkouts directory that are not marked.
		let mut read_dir = tokio::fs::read_dir(self.checkouts_path())
			.await
			.wrap_err("Failed to read the checkouts directory.")?;
		while let Some(entry) = read_dir.next_entry().await? {
			let artifact_hash: hash::Hash = entry
				.file_name()
				.to_str()
				.wrap_err("Failed to parse the file name as a string.")?
				.parse()
				.map_err(Error::other)
				.wrap_err("Failed to parse the entry in the checkouts directory as a hash.")?;
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

	fn sweep_package_instances(&self, marks: &Marks) -> Result<()> {
		// Open a read/write transaction.
		let mut txn = self.database.env.begin_rw_txn()?;

		// Open a read/write cursor.
		let mut cursor = txn.open_rw_cursor(self.database.package_instances)?;

		// Delete all packages that are not marked.
		for entry in cursor.iter() {
			let (hash, _) = entry?;
			let hash = hash.try_into().map_err(Error::other)?;
			let hash = package::instance::Hash(hash::Hash(hash));
			if !marks.contains_package(hash) {
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
	package_instances: HashSet<package::instance::Hash, hash::BuildHasher>,
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

	fn mark_package_instance(&mut self, package_instance_hash: package::instance::Hash) {
		self.package_instances.insert(package_instance_hash);
	}

	fn contains_package(&self, package_instance_hash: package::instance::Hash) -> bool {
		self.package_instances.contains(&package_instance_hash)
	}
}
