use super::State;
use crate::{
	db::ExpressionWithOutput,
	expression::{AddExpressionOutcome, Expression},
	hash::Hash,
	util::path_exists,
};
use anyhow::{bail, Context, Result};
use lmdb::{Cursor, Transaction};

impl State {
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
					let exists = self.expression_exists_local(hash)?;
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
				let exists = self.expression_exists_local(hash)?;
				if !exists {
					return Ok(AddExpressionOutcome::DependencyMissing { hash });
				}
			},

			// If this expression is a package, ensure its source and dependencies are present.
			Expression::Package(package) => {
				let hash = package.source;
				let exists = self.expression_exists_local(package.source)?;
				if !exists {
					missing.push(hash);
				}

				for hash in package.dependencies.values().copied() {
					if !self.expression_exists_local(hash)? {
						missing.push(hash);
					}
				}
			},

			// If this expression is null, there is nothing to ensure.
			Expression::Null(_) => {},

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
				for hash in template.components.iter().copied() {
					if !self.expression_exists_local(hash)? {
						missing.push(hash);
					}
				}
			},

			// If this expression is fetch, there is nothing to ensure.
			Expression::Fetch(_) => {},

			Expression::Js(js) => {
				// Ensure the artifact is present.
				let hash = js.package;
				let exists = self.expression_exists_local(hash)?;
				if !exists {
					missing.push(hash);
				}

				// Ensure the args are present.
				let hash = js.args;
				let exists = self.expression_exists_local(hash)?;
				if !exists {
					missing.push(hash);
				}
			},

			// If this expression is a process, ensure its children are present.
			Expression::Process(process) => {
				// Ensure the command expression is present.
				let hash = process.command;
				let exists = self.expression_exists_local(hash)?;
				if !exists {
					missing.push(hash);
				}

				// Ensure the args expression is present.
				let hash = process.args;
				let exists = self.expression_exists_local(hash)?;
				if !exists {
					missing.push(hash);
				}

				// Ensure the env expression is present.
				let hash = process.env;
				let exists = self.expression_exists_local(hash)?;
				if !exists {
					missing.push(hash);
				}
			},

			// If this expression is a target, ensure its children are present.
			Expression::Target(target) => {
				// Ensure the package is present.
				let hash = target.package;
				let exists = self.expression_exists_local(hash)?;
				if !exists {
					missing.push(hash);
				}

				// Ensure the args are present.
				let hash = target.args;
				let exists = self.expression_exists_local(hash)?;
				if !exists {
					missing.push(hash);
				}
			},

			// If this expression is an array, ensure the values are present.
			Expression::Array(array) => {
				for hash in array.iter().copied() {
					if !self.expression_exists_local(hash)? {
						missing.push(hash);
					}
				}
			},

			// If this expression is a map, ensure the values are present.
			Expression::Map(map) => {
				for hash in map.values().copied() {
					if !self.expression_exists_local(hash)? {
						missing.push(hash);
					}
				}
			},
		}

		// Return if there are any missing expressions.
		if !missing.is_empty() {
			return Ok(AddExpressionOutcome::MissingExpressions { hashes: missing });
		}

		// Hash the expression.
		let hash = expression.hash();

		// Serialize the expression with output.
		let value = ExpressionWithOutput {
			expression: expression.clone(),
			output_hash: None,
		};
		let value = buffalo::to_vec(&value).unwrap();

		// Get a write transaction.
		let mut txn = self.db.env.begin_rw_txn()?;

		// Add the expression to the database.
		match txn.put(
			self.db.expressions,
			&hash.as_slice(),
			&value,
			lmdb::WriteFlags::NO_OVERWRITE,
		) {
			Ok(_) => {},
			Err(lmdb::Error::KeyExist) => {},
			Err(error) => bail!(error),
		};

		// Commit the transaction.
		txn.commit()?;

		Ok(AddExpressionOutcome::Added { hash })
	}

	pub fn expression_exists_local(&self, hash: Hash) -> Result<bool> {
		// Get a read transaction.
		let txn = self.db.env.begin_ro_txn()?;

		let exists = match txn.get(self.db.expressions, &hash.as_slice()) {
			Ok(_) => Ok::<_, anyhow::Error>(true),
			Err(lmdb::Error::NotFound) => Ok(false),
			Err(error) => Err(error.into()),
		}?;

		Ok(exists)
	}

	pub fn get_expression_local(&self, hash: Hash) -> Result<Expression> {
		let expression = self
			.try_get_expression_local(hash)?
			.with_context(|| format!(r#"Failed to find the expression with hash "{hash}"."#))?;
		Ok(expression)
	}

	pub fn get_expression_local_with_txn<Txn>(&self, txn: &Txn, hash: Hash) -> Result<Expression>
	where
		Txn: lmdb::Transaction,
	{
		let expression = self
			.try_get_expression_local_with_txn(txn, hash)?
			.with_context(|| format!(r#"Failed to find the expression with hash "{hash}"."#))?;
		Ok(expression)
	}

	pub fn try_get_expression_local(&self, hash: Hash) -> Result<Option<Expression>> {
		// Get a read transaction.
		let txn = self.db.env.begin_ro_txn()?;

		// Get the expression.
		let maybe_expression = self.try_get_expression_local_with_txn(&txn, hash)?;

		Ok(maybe_expression)
	}

	pub fn try_get_expression_local_with_txn<Txn>(
		&self,
		txn: &Txn,
		hash: Hash,
	) -> Result<Option<Expression>>
	where
		Txn: lmdb::Transaction,
	{
		// Get the expression.
		let maybe_expression = self
			.try_get_expression_with_output_local_with_txn(txn, hash)?
			.map(|expression_with_output| expression_with_output.expression);

		Ok(maybe_expression)
	}

	pub fn get_expression_with_output_local(&self, hash: Hash) -> Result<ExpressionWithOutput> {
		let expression = self
			.try_get_expression_with_output_local(hash)?
			.with_context(|| format!(r#"Failed to find the expression with hash "{hash}"."#))?;
		Ok(expression)
	}

	pub fn get_expression_with_output_local_with_txn<Txn>(
		&self,
		txn: &Txn,
		hash: Hash,
	) -> Result<ExpressionWithOutput>
	where
		Txn: lmdb::Transaction,
	{
		let expression_with_output = self
			.try_get_expression_with_output_local_with_txn(txn, hash)?
			.with_context(|| format!(r#"Failed to find the expression with hash "{hash}"."#))?;
		Ok(expression_with_output)
	}

	pub fn try_get_expression_with_output_local(
		&self,
		hash: Hash,
	) -> Result<Option<ExpressionWithOutput>> {
		// Get a read transaction.
		let txn = self.db.env.begin_ro_txn()?;

		// Get the expression.
		let maybe_expression = self.try_get_expression_with_output_local_with_txn(&txn, hash)?;

		Ok(maybe_expression)
	}

	pub async fn try_get_expression_with_output_with_txn<Txn>(
		&self,
		txn: &Txn,
		hash: Hash,
	) -> Result<Option<ExpressionWithOutput>>
	where
		Txn: lmdb::Transaction,
	{
		// Get the expression from the local database.
		let maybe_expression = self.try_get_expression_with_output_local_with_txn(txn, hash)?;
		if let Some(expression) = maybe_expression {
			return Ok(Some(expression));
		}

		// Try to get the expression from the expression server.
		let maybe_expression = if let Some(expression_client) = &self.expression_client {
			expression_client
				.try_get_expression_with_output(hash)
				.await?
		} else {
			None
		};

		Ok(maybe_expression)
	}

	pub fn try_get_expression_with_output_local_with_txn<Txn>(
		&self,
		txn: &Txn,
		hash: Hash,
	) -> Result<Option<ExpressionWithOutput>>
	where
		Txn: lmdb::Transaction,
	{
		// Get the expression.
		let maybe_expression = match txn.get(self.db.expressions, &hash.as_slice()) {
			Ok(value) => {
				let value = buffalo::from_slice(value)?;
				Ok::<_, anyhow::Error>(Some(value))
			},
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => Err(error.into()),
		}?;

		Ok(maybe_expression)
	}

	pub fn add_evaluation(&self, parent_hash: Hash, child_hash: Hash) -> Result<()> {
		// Get a write transaction.
		let mut txn = self.db.env.begin_rw_txn()?;

		// Add the evaluation.
		txn.put(
			self.db.evaluations,
			&parent_hash.as_slice(),
			&child_hash.as_slice(),
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		Ok(())
	}

	pub fn get_evaluations_with_txn<Txn>(
		&self,
		txn: &Txn,
		hash: Hash,
	) -> Result<impl Iterator<Item = Result<Hash>>>
	where
		Txn: lmdb::Transaction,
	{
		// Get a cursor.
		let mut cursor = txn.open_ro_cursor(self.db.evaluations)?;

		// Get the evaluations.
		let evaluations =
			cursor
				.iter_dup_of(&hash.as_slice())
				.into_iter()
				.map(|value| match value {
					Ok((_, value)) => {
						let value = buffalo::from_slice(value)?;
						Ok(value)
					},
					Err(error) => Err(error.into()),
				});

		Ok(evaluations)
	}

	/// Add an expression with output to the database.
	pub fn set_expression_output(&self, hash: Hash, output_hash: Hash) -> Result<()> {
		// Get a write transaction.
		let mut txn = self.db.env.begin_rw_txn()?;

		// Get the expression.
		let expression = self.get_expression_local_with_txn(&txn, hash)?;

		// Create the expression with output.
		let value = ExpressionWithOutput {
			expression,
			output_hash: Some(output_hash),
		};
		let value = buffalo::to_vec(&value).unwrap();

		// Add the expression with output to the database.
		txn.put(
			self.db.expressions,
			&hash.as_slice(),
			&value,
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		Ok(())
	}
}
