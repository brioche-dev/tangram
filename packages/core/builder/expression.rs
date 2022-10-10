use super::Shared;
use crate::{
	expression::{AddExpressionOutcome, Expression},
	hash::Hash,
	util::path_exists,
};
use anyhow::{anyhow, bail, Context, Result};
use futures::stream::TryStreamExt;

impl Shared {
	pub async fn add_expression(&self, expression: &Expression) -> Result<Hash> {
		match self.try_add_expression(expression).await? {
			AddExpressionOutcome::Added { hash } => Ok(hash),
			_ => bail!("Failed to add the expression."),
		}
	}

	// Add an expression to the server after ensuring the server has all its references.
	#[allow(clippy::too_many_lines, clippy::match_same_arms)]
	pub async fn try_add_expression(
		&self,
		expression: &Expression,
	) -> Result<AddExpressionOutcome> {
		// Before adding this expression, we need to ensure the builder has all its references.
		let mut missing = Vec::new();
		match expression {
			// If this expression is a directory, ensure all its entries are present.
			Expression::Directory(directory) => {
				let mut missing = Vec::new();
				for (entry_name, hash) in &directory.entries {
					let hash = *hash;
					let exists = self.expression_exists(hash)?;
					if !exists {
						missing.push((entry_name.clone(), hash));
					}
				}
				if !missing.is_empty() {
					return Ok(AddExpressionOutcome::DirectoryMissingEntries { entries: missing });
				}
			},

			// If this expression is a file, ensure its blob is present.
			Expression::File(file) => {
				let blob_path = self.blob_path(file.blob);
				let blob_exists = path_exists(&blob_path).await?;
				if !blob_exists {
					return Ok(AddExpressionOutcome::FileMissingBlob {
						blob_hash: file.blob,
					});
				}
			},

			// If this expression is a symlink, there is nothing to ensure.
			Expression::Symlink(_) => {},

			// If this expression is a dependency, ensure the dependency is present.
			Expression::Dependency(dependency) => {
				let hash = dependency.artifact;
				let exists = self.expression_exists(hash)?;
				if !exists {
					return Ok(AddExpressionOutcome::DependencyMissing { hash });
				}
			},

			// If this expression is a package, ensure its source and dependencies are present.
			Expression::Package(package) => {
				let hash = package.source;
				let exists = self.expression_exists(package.source)?;
				if !exists {
					missing.push(hash);
				}
				missing.extend(
					futures::stream::iter(
						package
							.dependencies
							.values()
							.copied()
							.map(Ok::<_, anyhow::Error>),
					)
					.try_filter_map(|hash| async move {
						let exists = self.expression_exists(hash)?;
						if exists {
							Ok(None)
						} else {
							Ok(Some(hash))
						}
					})
					.try_collect::<Vec<Hash>>()
					.await?,
				);
			},

			// If this expression is null, there is nothing to ensure.
			Expression::Null => {},

			// If this expression is bool, there is nothing to ensure.
			Expression::Bool(_) => {},

			// If this expression is number, there is nothing to ensure.
			Expression::Number(_) => {},

			// If this expression is string, there is nothing to ensure.
			Expression::String(_) => {},

			// If this expression is artifact, there is nothing to ensure.
			Expression::Artifact(_) => {},

			// If this expression is a template, ensure the components are present.
			Expression::Template(template) => {
				missing.extend(
					futures::stream::iter(
						template
							.components
							.iter()
							.copied()
							.map(Ok::<_, anyhow::Error>),
					)
					.try_filter_map(|hash| async move {
						let exists = self.expression_exists(hash)?;
						if exists {
							Ok(None)
						} else {
							Ok(Some(hash))
						}
					})
					.try_collect::<Vec<Hash>>()
					.await?,
				);
			},

			// If this expression is fetch, there is nothing to ensure.
			Expression::Fetch(_) => {},

			Expression::Js(js) => {
				// Ensure the artifact is present.
				let hash = js.package;
				let exists = self.expression_exists(hash)?;
				if !exists {
					missing.push(hash);
				}

				// Ensure the args are present.
				let hash = js.args;
				let exists = self.expression_exists(hash)?;
				if !exists {
					missing.push(hash);
				}
			},

			// If this expression is a process, ensure its children are present.
			Expression::Process(process) => {
				// Ensure the command expression is present.
				let hash = process.command;
				let exists = self.expression_exists(hash)?;
				if !exists {
					missing.push(hash);
				}

				// Ensure the args expression is present.
				let hash = process.args;
				let exists = self.expression_exists(hash)?;
				if !exists {
					missing.push(hash);
				}

				// Ensure the env expression is present.
				let hash = process.env;
				let exists = self.expression_exists(hash)?;
				if !exists {
					missing.push(hash);
				}
			},

			// If this expression is a target, ensure its children are present.
			Expression::Target(target) => {
				// Ensure the package is present.
				let hash = target.package;
				let exists = self.expression_exists(hash)?;
				if !exists {
					missing.push(hash);
				}

				// Ensure the args are present.
				let hash = target.args;
				let exists = self.expression_exists(hash)?;
				if !exists {
					missing.push(hash);
				}
			},

			// If this expression is an array, ensure the values are present.
			Expression::Array(array) => {
				missing.extend(
					futures::stream::iter(array.iter().copied().map(Ok::<_, anyhow::Error>))
						.try_filter_map(|hash| async move {
							let exists = self.expression_exists(hash)?;
							if exists {
								Ok(None)
							} else {
								Ok(Some(hash))
							}
						})
						.try_collect::<Vec<Hash>>()
						.await?,
				);
			},

			// If this expression is a map, ensure the values are present.
			Expression::Map(map) => {
				missing.extend(
					futures::stream::iter(map.values().copied().map(Ok::<_, anyhow::Error>))
						.try_filter_map(|hash| async move {
							let exists = self.expression_exists(hash)?;
							if exists {
								Ok(None)
							} else {
								Ok(Some(hash))
							}
						})
						.try_collect::<Vec<Hash>>()
						.await?,
				);
			},
		}

		// If there are any missing expressions, return.
		if !missing.is_empty() {
			return Ok(AddExpressionOutcome::MissingExpressions { hashes: missing });
		}

		// Serialize and hash the expression.
		let data = serde_json::to_vec(&expression)?;
		let hash = Hash::new(&data);

		// Add the expression to the database.
		let mut txn = self
			.env
			.write_txn()
			.map_err(|_| anyhow!("Unable to get a write transaction"))?;

		self.expressions_db
			.put(&mut txn, &hash, &(expression.clone(), None))
			.map_err(|_| anyhow!("Unable to put the expression"))?;

		txn.commit()
			.map_err(|_| anyhow!("Unable to commit the transaction."))?;

		Ok(AddExpressionOutcome::Added { hash })
	}

	pub fn expression_exists(&self, hash: Hash) -> Result<bool> {
		let txn = self
			.env
			.read_txn()
			.map_err(|_| anyhow!("Unable to get a read transaction"))?;

		let exists = self
			.expressions_db
			.get(&txn, &hash)
			.map_err(|_| anyhow!("Unable to get the value."))?
			.is_some();

		Ok(exists)
	}

	pub fn get_expression(&self, hash: Hash) -> Result<Expression> {
		let expression = self
			.try_get_expression(hash)?
			.with_context(|| format!(r#"Failed to find the expression with hash "{hash}"."#))?;
		Ok(expression)
	}

	pub fn try_get_expression(&self, hash: Hash) -> Result<Option<Expression>> {
		let txn = self
			.env
			.read_txn()
			.map_err(|_| anyhow!("Unable to get a read transaction"))?;

		let maybe_expression = self
			.expressions_db
			.get(&txn, &hash)
			.map_err(|_| anyhow!("Unable to get the value."))?
			.map(|(expression, _)| expression);

		Ok(maybe_expression)
	}

	pub fn get_expression_with_output(&self, hash: Hash) -> Result<(Expression, Option<Hash>)> {
		let expression = self
			.try_get_expression_with_output(hash)?
			.with_context(|| format!(r#"Failed to find the expression with hash "{hash}"."#))?;
		Ok(expression)
	}

	pub fn try_get_expression_with_output(
		&self,
		hash: Hash,
	) -> Result<Option<(Expression, Option<Hash>)>> {
		let txn = self
			.env
			.read_txn()
			.map_err(|_| anyhow!("Unable to get a read transaction"))?;

		let maybe_expression = self
			.expressions_db
			.get(&txn, &hash)
			.map_err(|_| anyhow!("Unable to get the value."))?;

		Ok(maybe_expression)
	}

	pub fn delete_expression(&self, hash: Hash) -> Result<()> {
		let mut txn = self
			.env
			.write_txn()
			.map_err(|_| anyhow!("Unable to get a write transaction"))?;

		self.expressions_db
			.delete(&mut txn, &hash)
			.map_err(|_| anyhow!("Unable to delete the expression."))?;

		Ok(())
	}

	pub fn add_evaluation(&self, parent_hash: Hash, child_hash: Hash) -> Result<()> {
		let mut txn = self
			.env
			.write_txn()
			.map_err(|_| anyhow!("Unable to get a write transaction"))?;

		let mut children = self
			.evaluations_db
			.get(&txn, &parent_hash)
			.map_err(|_| anyhow!("Unable to get."))?
			.unwrap_or_default();

		children.push(child_hash);

		self.evaluations_db
			.put(&mut txn, &parent_hash, &children)
			.map_err(|_| anyhow!("Unable to delete the expression."))?;

		txn.commit()
			.map_err(|_| anyhow!("Unable to commit the transaction."))?;

		Ok(())
	}

	pub fn get_evaluations(&self, hash: Hash) -> Result<Vec<Hash>> {
		let txn = self
			.env
			.read_txn()
			.map_err(|_| anyhow!("Unable to get a read transaction"))?;

		let children = self
			.evaluations_db
			.get(&txn, &hash)
			.map_err(|_| anyhow!("Unable to get evaluations."))?
			.unwrap_or_default();

		Ok(children)
	}

	/// Memoize the output from the evaluation of an expression.
	pub fn set_expression_output(&self, hash: Hash, output_hash: Hash) -> Result<()> {
		let mut txn = self
			.env
			.write_txn()
			.map_err(|_| anyhow!("Unable to get a write transaction"))?;

		// Get the expression.
		let expression = self.get_expression(hash)?;

		// Set the expression output.
		self.expressions_db
			.put(&mut txn, &hash, &(expression, Some(output_hash)))
			.map_err(|_| anyhow!("Unable to set the expression output."))?;

		txn.commit()
			.map_err(|_| anyhow!("Unable to commit the transaction."))?;

		Ok(())
	}
}
