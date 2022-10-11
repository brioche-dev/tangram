use super::Exclusive;
use crate::{expression::Expression, hash::Hash};
use anyhow::{bail, Context, Result};
use lmdb::{Cursor, Transaction};
use std::{
	collections::{HashSet, VecDeque},
	path::Path,
};

impl Exclusive {
	pub async fn garbage_collect(&self, roots: Vec<Hash>) -> Result<()> {
		// Create hash sets to track the marked expressions and blobs.
		let mut marked_hashes: HashSet<Hash, fnv::FnvBuildHasher> = HashSet::default();
		let mut marked_blob_hashes: HashSet<Hash, fnv::FnvBuildHasher> = HashSet::default();

		// Mark the expressions and blobs.
		self.mark(&mut marked_hashes, &mut marked_blob_hashes, roots)
			.context("Failed to mark the expressions and blobs.")?;

		// Sweep the expressions.
		self.sweep_expressions(&marked_hashes)
			.context("Failed to sweep the expressions.")?;

		// Sweep the blobs.
		self.sweep_blobs(&self.as_shared().blobs_path(), &marked_blob_hashes)
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
		marked_hashes: &mut HashSet<Hash, fnv::FnvBuildHasher>,
		marked_blob_hashes: &mut HashSet<Hash, fnv::FnvBuildHasher>,
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
			let (expression, output_hash) =
				self.as_shared().get_expression_local_with_output(hash)?;

			// Add this expression's output, if it has one, to the queue.
			if let Some(output_hash) = output_hash {
				queue.push_back(output_hash);
			}

			// Add this expression's evaluations to the queue.
			let child_hashes = self.as_shared().get_evaluations(hash)?;
			queue.extend(child_hashes);

			// Add the expression's children to the queue.
			match expression {
				Expression::Null
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

	fn sweep_expressions(&self, marked_hashes: &HashSet<Hash, fnv::FnvBuildHasher>) -> Result<()> {
		// Get a read transaction.
		let txn = self.env.begin_ro_txn()?;

		// Get a read cursor.
		let mut cursor = txn.open_ro_cursor(self.expressions_db)?;

		// Get an iterator over all expressions.
		let hashes = cursor
			.iter()
			.map(|value| match value {
				Ok((key, _)) => {
					let key: Hash = serde_json::from_slice(key)?;
					Ok(key)
				},
				Err(e) => bail!(e),
			})
			.collect::<Result<Vec<Hash>>>()?;

		// Delete all expressions that are not marked.
		for hash in hashes {
			if !marked_hashes.contains(&hash) {
				self.as_shared().delete_expression(hash)?;
			}
		}

		Ok(())
	}

	async fn sweep_blobs(
		&self,
		blobs_path: &Path,
		marked_blob_hashes: &HashSet<Hash, fnv::FnvBuildHasher>,
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
}
