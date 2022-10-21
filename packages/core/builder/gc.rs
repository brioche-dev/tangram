use super::State;
use crate::{
	db::ExpressionWithOutput,
	expression::Expression,
	hash::{self, Hash},
};
use anyhow::{bail, Context, Result};
use lmdb::{Cursor, Transaction};
use std::{
	collections::{HashSet, VecDeque},
	path::Path,
};

impl State {
	pub async fn garbage_collect(&mut self, roots: Vec<Hash>) -> Result<()> {
		// Create hash sets to track the marked expressions and blobs.
		let mut marked_hashes: HashSet<Hash, hash::BuildHasher> = HashSet::default();
		let mut marked_blob_hashes: HashSet<Hash, hash::BuildHasher> = HashSet::default();

		{
			// Create a read/write transaction.
			let mut txn = self.db.env.begin_rw_txn()?;

			// Mark the expressions and blobs.
			self.mark(&mut txn, &mut marked_hashes, &mut marked_blob_hashes, roots)
				.context("Failed to mark the expressions and blobs.")?;

			// Sweep the expressions.
			self.sweep_expressions_with_txn(&mut txn, &marked_hashes)
				.context("Failed to sweep the expressions.")?;

			// Commit.
			txn.commit()?;
		}

		// Sweep the artifacts.
		self.sweep_artifacts(&self.artifacts_path(), &marked_hashes)
			.await
			.context("Failed to sweep the artifacts.")?;

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

		Ok(())
	}

	#[allow(clippy::too_many_lines)]
	fn mark(
		&self,
		txn: &mut lmdb::RwTransaction,
		marked_hashes: &mut HashSet<Hash, hash::BuildHasher>,
		marked_blob_hashes: &mut HashSet<Hash, hash::BuildHasher>,
		roots: Vec<Hash>,
	) -> Result<()> {
		// Traverse the transitive dependencies of the roots and add each hash to the marked expression hashes and marked blob hashes.

		let mut queue: VecDeque<Hash> = VecDeque::from_iter(roots);
		while let Some(hash) = queue.pop_front() {
			// If this expression has already been marked, continue to avoid an infinite loop.
			if marked_hashes.contains(&hash) {
				continue;
			}

			// Mark this expression.
			marked_hashes.insert(hash);

			// Get the expression.
			let ExpressionWithOutput {
				expression,
				output_hash,
			} = self.get_expression_with_output_local_with_txn(txn, hash)?;

			// Add this expression's output, if it has one, to the queue.
			if let Some(output_hash) = output_hash {
				queue.push_back(output_hash);
			}

			// Add this expression's evaluations to the queue.
			let hashes = self.get_evaluations_with_txn(txn, hash)?;
			for hash in hashes {
				let hash = hash?;
				queue.push_back(hash);
			}

			// Add the expression's children to the queue.
			match expression {
				Expression::Null(_)
				| Expression::Bool(_)
				| Expression::Number(_)
				| Expression::String(_)
				| Expression::Fetch(_)
				| Expression::Symlink(_) => {},

				Expression::Artifact(artifact) => {
					queue.push_back(artifact.root);
				},

				// If the expression is a file, mark its blob.
				Expression::File(file) => {
					marked_blob_hashes.insert(file.blob);
				},

				// If the expression is a directory, add its entries to the queue.
				Expression::Directory(directory) => {
					for (_, hash) in directory.entries {
						queue.push_back(hash);
					}
				},

				// If the expression is a dependency, add the dependent expression to the queue.
				Expression::Dependency(dependency) => {
					queue.push_back(dependency.artifact);
				},

				Expression::Package(package) => {
					queue.push_back(package.source);
					queue.extend(package.dependencies.values());
				},

				Expression::Template(template) => {
					queue.extend(template.components);
				},

				Expression::Js(js) => {
					queue.push_back(js.package);
					queue.push_back(js.args);
				},

				Expression::Process(process) => {
					queue.push_back(process.env);
					queue.push_back(process.command);
					queue.push_back(process.args);
				},

				Expression::Target(target) => {
					queue.push_back(target.package);
					queue.push_back(target.args);
				},

				Expression::Array(array) => {
					queue.extend(array);
				},

				Expression::Map(map) => {
					for (_, value) in map {
						queue.push_back(value);
					}
				},
			}
		}

		Ok(())
	}

	fn sweep_expressions_with_txn(
		&self,
		txn: &mut lmdb::RwTransaction,
		marked_hashes: &HashSet<Hash, hash::BuildHasher>,
	) -> Result<()> {
		// Get a read cursor.
		let mut cursor = txn.open_rw_cursor(self.db.expressions)?;

		// Get an iterator over all expressions and delete them if they are not marked.
		for value in cursor.iter() {
			match value {
				Ok((key, _)) => {
					let key = key.try_into()?;
					let hash = Hash(key);
					if !marked_hashes.contains(&hash) {
						cursor.del(lmdb::WriteFlags::empty())?;
					}
				},
				Err(error) => bail!(error),
			}
		}

		Ok(())
	}

	async fn sweep_blobs(
		&self,
		blobs_path: &Path,
		marked_blob_hashes: &HashSet<Hash, hash::BuildHasher>,
	) -> Result<()> {
		// Delete all blobs that are not not marked.
		let mut read_dir = tokio::fs::read_dir(blobs_path)
			.await
			.context("Failed to read the blobs directory.")?;
		while let Some(entry) = read_dir.next_entry().await? {
			let blob_hash: Hash = entry
				.file_name()
				.to_str()
				.context("Failed to parse the file name as a string.")?
				.parse()
				.context("Failed to parse the entry in the blobs directory as a hash.")?;
			if !marked_blob_hashes.contains(&blob_hash) {
				tokio::fs::remove_file(&entry.path())
					.await
					.context("Failed to remove the blob.")?;
			}
		}
		Ok(())
	}

	async fn sweep_artifacts(
		&self,
		artifacts_path: &Path,
		marked_hashes: &HashSet<Hash, hash::BuildHasher>,
	) -> Result<()> {
		// Delete all artifacts that are not not marked.
		let mut read_dir = tokio::fs::read_dir(artifacts_path)
			.await
			.context("Failed to read the artifacts directory.")?;
		while let Some(entry) = read_dir.next_entry().await? {
			let artifact_hash: Hash = entry
				.file_name()
				.to_str()
				.context("Failed to parse the file name as a string.")?
				.parse()
				.context("Failed to parse the entry in the artifacts directory as a hash.")?;
			if !marked_hashes.contains(&artifact_hash) {
				tokio::fs::remove_file(&entry.path())
					.await
					.context("Failed to remove the artifact.")?;
			}
		}
		Ok(())
	}
}
