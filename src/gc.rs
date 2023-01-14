use crate::{
	artifact::{Artifact, ArtifactHash},
	blob::BlobHash,
	hash::{self, Hash},
	operation::{Operation, OperationHash},
	package::PackageHash,
	util::rmrf,
	value::{TemplateComponent, Value},
	Cli,
};
use anyhow::{Context, Result};
use lmdb::{Cursor, Transaction};
use std::collections::{HashSet, VecDeque};

impl Cli {
	pub async fn garbage_collect(&self, roots: Vec<OperationHash>) -> Result<()> {
		// Create marks to track marked artifacts, blobs, operations, and packages.
		let mut marks = Marks::default();

		// Mark the artifacts, blobs, operations, and packages.
		self.mark(&mut marks, roots)
			.context("Failed to perform the mark phase.")?;

		// Sweep the artifacts.
		self.sweep_artifacts(&marks)
			.context("Failed to sweep the artifacts.")?;

		// Sweep the checkouts directory.
		self.sweep_checkouts_directory(&marks)
			.await
			.context("Failed to sweep the checkouts directory.")?;

		// Sweep the blobs.
		self.sweep_blobs(&marks)
			.await
			.context("Failed to sweep the blobs.")?;

		// Sweep the operations.
		self.sweep_operations(&marks)
			.context("Failed to sweep operations.")?;

		// Sweep the operation's children.
		self.sweep_operation_children(&marks)
			.context("Failed to sweep operation children.")?;

		// Sweep the packages.
		self.sweep_packages(&marks)
			.context("Failed to sweep packages.")?;

		// Delete all temps.
		tokio::fs::remove_dir_all(&self.temps_path())
			.await
			.context("Failed to delete the temps directory.")?;
		tokio::fs::create_dir_all(&self.temps_path())
			.await
			.context("Failed to recreate the temps directory.")?;

		// TODO: Compact the database.

		Ok(())
	}
}

enum QueueItem {
	Value(Value),
	Artifact(ArtifactHash),
	Operation(OperationHash),
	Package(PackageHash),
}

impl Cli {
	#[allow(clippy::too_many_lines)]
	fn mark(&self, marks: &mut Marks, roots: Vec<OperationHash>) -> Result<()> {
		let txn = self.inner.database.env.begin_ro_txn()?;
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
							marks.mark_blob(file.blob);
						},

						Artifact::Symlink(_) => {},

						Artifact::Dependency(dependency) => {
							// Add the artifact to the queue.
							queue.push_back(QueueItem::Artifact(dependency.artifact));
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
								TemplateComponent::String(_)
								| TemplateComponent::Placeholder(_) => {},
								TemplateComponent::Artifact(artifact) => {
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
							if let Some(env) = process.env {
								for (_, template) in env {
									queue.push_back(QueueItem::Value(Value::Template(template)));
								}
							}

							// Add the command to the queue.
							queue.push_back(QueueItem::Value(Value::Template(process.command)));

							// Add the args to the queue.
							if let Some(args) = process.args {
								for template in args {
									queue.push_back(QueueItem::Value(Value::Template(template)));
								}
							}
						},

						Operation::Target(target) => {
							// Add the package to the queue.
							queue.push_back(QueueItem::Package(target.package));

							// Add the args to the queue.
							for value in target.args {
								queue.push_back(QueueItem::Value(value));
							}
						},
					}
				},

				QueueItem::Package(package_hash) => {
					// Mark the package.
					marks.mark_package(package_hash);

					// Get the package.
					let package = self.get_package_local_with_txn(&txn, package_hash)?;

					// Add the source to the queue.
					queue.push_back(QueueItem::Artifact(package.source));

					// Mark the package's dependencies.
					for dependency in package.dependencies.into_values() {
						marks.mark_package(dependency);
					}
				},
			}
		}

		Ok(())
	}
}

impl Cli {
	fn sweep_artifacts(&self, marks: &Marks) -> Result<()> {
		// Open a read/write transaction.
		let mut txn = self.inner.database.env.begin_rw_txn()?;

		// Open a read/write cursor.
		let mut cursor = txn.open_rw_cursor(self.inner.database.artifacts)?;

		// Delete all artifacts that are not marked.
		for entry in cursor.iter() {
			let (hash, _) = entry?;
			let hash = hash.try_into()?;
			let hash = ArtifactHash(Hash(hash));
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
			.context("Failed to read the checkouts directory.")?;
		while let Some(entry) = read_dir.next_entry().await? {
			let artifact_hash: Hash = entry
				.file_name()
				.to_str()
				.context("Failed to parse the file name as a string.")?
				.parse()
				.context("Failed to parse the entry in the checkouts directory as a hash.")?;
			let artifact_hash = ArtifactHash(artifact_hash);
			if !marks.contains_artifact(artifact_hash) {
				rmrf(&entry.path(), None)
					.await
					.context("Failed to remove the artifact.")?;
			}
		}
		Ok(())
	}

	async fn sweep_blobs(&self, marks: &Marks) -> Result<()> {
		// Delete all blobs that are not not marked.
		let mut read_dir = tokio::fs::read_dir(self.blobs_path())
			.await
			.context("Failed to read the blobs directory.")?;
		while let Some(entry) = read_dir.next_entry().await? {
			let blob_hash: Hash = entry
				.file_name()
				.to_str()
				.context("Failed to parse the file name as a string.")?
				.parse()
				.context("Failed to parse the entry in the blobs directory as a hash.")?;
			let blob_hash = BlobHash(blob_hash);
			if !marks.contains_blob(blob_hash) {
				tokio::fs::remove_file(&entry.path())
					.await
					.context("Failed to remove the blob.")?;
			}
		}
		Ok(())
	}

	fn sweep_operations(&self, marks: &Marks) -> Result<()> {
		// Open a read/write transaction.
		let mut txn = self.inner.database.env.begin_rw_txn()?;

		// Open a read/write cursor.
		let mut cursor = txn.open_rw_cursor(self.inner.database.operations)?;

		// Delete all operations that are not marked.
		for entry in cursor.iter() {
			let (hash, _) = entry?;
			let hash = hash.try_into()?;
			let hash = OperationHash(Hash(hash));
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
		let mut txn = self.inner.database.env.begin_rw_txn()?;

		// Open a read/write cursor.
		let mut cursor = txn.open_rw_cursor(self.inner.database.operation_children)?;

		// Delete all operations that are not marked.
		for entry in cursor.iter() {
			let (hash, _) = entry?;
			let hash = hash.try_into()?;
			let hash = OperationHash(Hash(hash));
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

	fn sweep_packages(&self, marks: &Marks) -> Result<()> {
		// Open a read/write transaction.
		let mut txn = self.inner.database.env.begin_rw_txn()?;

		// Open a read/write cursor.
		let mut cursor = txn.open_rw_cursor(self.inner.database.packages)?;

		// Delete all packages that are not marked.
		for entry in cursor.iter() {
			let (hash, _) = entry?;
			let hash = hash.try_into()?;
			let hash = PackageHash(Hash(hash));
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
	artifacts: HashSet<ArtifactHash, hash::BuildHasher>,
	blobs: HashSet<BlobHash, hash::BuildHasher>,
	operations: HashSet<OperationHash, hash::BuildHasher>,
	packages: HashSet<PackageHash, hash::BuildHasher>,
}

impl Marks {
	fn mark_artifact(&mut self, artifact_hash: ArtifactHash) {
		self.artifacts.insert(artifact_hash);
	}

	fn contains_artifact(&self, artifact_hash: ArtifactHash) -> bool {
		self.artifacts.contains(&artifact_hash)
	}

	fn mark_blob(&mut self, blob_hash: BlobHash) {
		self.blobs.insert(blob_hash);
	}

	fn contains_blob(&self, blob_hash: BlobHash) -> bool {
		self.blobs.contains(&blob_hash)
	}

	fn mark_operation(&mut self, operation_hash: OperationHash) {
		self.operations.insert(operation_hash);
	}

	fn contains_operation(&self, operation_hash: OperationHash) -> bool {
		self.operations.contains(&operation_hash)
	}

	fn mark_package(&mut self, package_hash: PackageHash) {
		self.packages.insert(package_hash);
	}

	fn contains_package(&self, package_hash: PackageHash) -> bool {
		self.packages.contains(&package_hash)
	}
}
