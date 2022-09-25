use super::Server;
use crate::{expression::Expression, hash::Hash};
use anyhow::{Context, Result};
use std::{
	collections::{HashSet, VecDeque},
	path::Path,
	sync::Arc,
};

impl Server {
	pub async fn garbage_collect(self: &Arc<Self>) -> Result<()> {
		// Acquire an exclusive lock to the path.
		let _path_lock_guard = self.lock.lock_exclusive().await?;

		// Create hash sets to track the marked expressions and blobs.
		let mut marked_hashes: HashSet<Hash, fnv::FnvBuildHasher> = HashSet::default();
		let mut marked_blob_hashes: HashSet<Hash, fnv::FnvBuildHasher> = HashSet::default();

		self.database_transaction(|txn| {
			// Mark the expressions and blobs.
			Self::mark(txn, &mut marked_hashes, &mut marked_blob_hashes)
				.context("Failed to mark the expressions and blobs.")?;

			// Sweep the expressions.
			Self::sweep_expressions(txn, &marked_hashes)
				.context("Failed to sweep the expressions.")?;

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

		Ok(())
	}

	#[allow(clippy::too_many_lines)]
	fn mark(
		txn: &rusqlite::Transaction,
		marked_hashes: &mut HashSet<Hash, fnv::FnvBuildHasher>,
		marked_blob_hashes: &mut HashSet<Hash, fnv::FnvBuildHasher>,
	) -> Result<()> {
		// Get the roots.
		let sql = r#"
			select
				hash
			from
				roots
		"#;
		let mut statement = txn
			.prepare_cached(sql)
			.context("Failed to prepare the query.")?;
		let roots = statement
			.query(())
			.context("Failed to execute the query.")?
			.and_then(|row| {
				let hash = row.get::<_, String>(0)?;
				let hash = hash.parse().with_context(|| "Failed to parse the hash.")?;
				Ok::<_, anyhow::Error>(hash)
			});

		// Traverse the transitive dependencies of the roots and add each hash to the marked expression hashes and marked blob hashes.
		let mut queue: VecDeque<Hash> = VecDeque::new();
		for root in roots {
			let root = root?;
			queue.push_back(root);
		}
		while let Some(hash) = queue.pop_front() {
			// If this expression has already been marked, continue to avoid an infinite loop.
			if marked_hashes.contains(&hash) {
				continue;
			}

			// Mark this expression.
			marked_hashes.insert(hash);

			// Get the expression.
			let (expression, output_hash) =
				Self::get_expression_with_output_with_transaction(txn, hash)?;

			// Add this expression's output, if it has one, to the queue.
			if let Some(output_hash) = output_hash {
				queue.push_back(output_hash);
			}

			// Add this expression's evaluations to the queue.
			let sql = r#"
				select
					child_hash
				from
					evaluations
				where
					parent_hash = $1
			"#;
			let params = (hash.to_string(),);
			let mut statement = txn
				.prepare_cached(sql)
				.context("Failed to prepare the query.")?;
			let evaluations = statement
				.query(params)
				.context("Failed to execute the query.")?
				.and_then(|row| {
					let hash = row.get::<_, String>(0)?;
					let hash = hash.parse().with_context(|| "Failed to parse the hash.")?;
					Ok::<_, anyhow::Error>(hash)
				});
			for hash in evaluations {
				let hash = hash?;
				queue.push_back(hash);
			}

			// Add the expression's children to the queue.
			match expression {
				Expression::Null
				| Expression::Bool(_)
				| Expression::Number(_)
				| Expression::String(_)
				| Expression::Fetch(_)
				| Expression::Symlink(_) => {},

				Expression::Artifact(artifact) => {
					queue.push_back(artifact.hash);
				},

				// If the expression is a file, mark its blob.
				Expression::File(file) => {
					marked_blob_hashes.insert(file.hash);
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

				Expression::Path(path) => {
					queue.push_back(path.artifact);
				},

				Expression::Template(template) => {
					queue.extend(template.components);
				},

				Expression::Process(process) => match process {
					crate::expression::Process::Amd64Linux(process)
					| crate::expression::Process::Amd64Macos(process)
					| crate::expression::Process::Arm64Linux(process)
					| crate::expression::Process::Arm64Macos(process) => {
						queue.push_back(process.env);
						queue.push_back(process.command);
						queue.push_back(process.args);
						for (_, output) in process.outputs {
							for (_, dependency) in output.dependencies {
								queue.push_back(dependency);
							}
						}
					},
					crate::expression::Process::Js(process) => {
						queue.push_back(process.module);
						queue.push_back(process.args);
					},
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

	fn sweep_expressions(
		txn: &rusqlite::Transaction,
		marked_hashes: &HashSet<Hash, fnv::FnvBuildHasher>,
	) -> Result<()> {
		// Get an iterator over all expressions.
		let sql = r#"
			select
				hash
			from
				expressions
		"#;
		let mut statement = txn
			.prepare_cached(sql)
			.context("Failed to prepare the query.")?;
		let hashes = statement
			.query(())
			.context("Failed to execute the query.")?
			.and_then(|row| {
				let hash = row.get::<_, String>(0)?;
				let hash = hash.parse().with_context(|| "Failed to parse the hash.")?;
				Ok::<_, anyhow::Error>(hash)
			});

		// Delete all expressions that are not marked.
		for hash in hashes {
			let hash = hash?;
			if !marked_hashes.contains(&hash) {
				Self::delete_expression_with_transaction(txn, hash)?;
			}
		}

		Ok(())
	}

	async fn sweep_blobs(
		self: &Arc<Self>,
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
