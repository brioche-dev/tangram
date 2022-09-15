use super::{error::bad_request, Server};
use crate::{expression::Expression, hash::Hash, util::path_exists};
use anyhow::{anyhow, bail, Context, Result};
use async_recursion::async_recursion;
use futures::{
	future::{select_ok, FutureExt},
	stream::TryStreamExt,
};
use std::sync::Arc;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AddExpressionOutcome {
	Added { hash: Hash },
	DirectoryMissingEntries { entries: Vec<(String, Hash)> },
	FileMissingBlob { blob_hash: Hash },
	DependencyMissing { hash: Hash },
	MissingExpressions { hashes: Vec<Hash> },
}

impl Server {
	pub async fn add_expression(self: &Arc<Self>, expression: &Expression) -> Result<Hash> {
		match self.try_add_expression(expression).await? {
			AddExpressionOutcome::Added { hash } => Ok(hash),
			_ => bail!("Failed to add the expression."),
		}
	}

	// Add an expression to the server after ensuring the server has all its references.
	#[allow(clippy::too_many_lines, clippy::match_same_arms)]
	pub async fn try_add_expression(
		self: &Arc<Self>,
		expression: &Expression,
	) -> Result<AddExpressionOutcome> {
		// Before adding this expression, we need to ensure the server has all its references.
		let mut missing = Vec::new();
		match expression {
			// If this expression is a directory, ensure all its entries are present.
			Expression::Directory(directory) => {
				let mut missing = Vec::new();
				for (entry_name, hash) in &directory.entries {
					let hash = *hash;
					let exists = self.expression_exists(hash).await?;
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
				let blob_path = self.blob_path(file.blob_hash);
				let blob_exists = path_exists(&blob_path).await?;
				if !blob_exists {
					return Ok(AddExpressionOutcome::FileMissingBlob {
						blob_hash: file.blob_hash,
					});
				}
			},

			// If this expression is a symlink, there is nothing to ensure.
			Expression::Symlink(_) => {},

			// If this expression is a dependency, ensure the dependency is present.
			Expression::Dependency(dependency) => {
				let hash = dependency.artifact.hash;
				let exists = self.expression_exists(hash).await?;
				if !exists {
					return Ok(AddExpressionOutcome::DependencyMissing { hash });
				}
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

			// If this expression is a path, ensure the artifact is present.
			Expression::Path(path) => {
				let hash = path.artifact;
				let exists = self.expression_exists(hash).await?;
				if !exists {
					return Ok(AddExpressionOutcome::DependencyMissing { hash });
				}
			},

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
						let exists = self.expression_exists(hash).await?;
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

			// If this expression is a process, ensure its children are present.
			Expression::Process(process) => match process {
				crate::expression::Process::Amd64Linux(process)
				| crate::expression::Process::Amd64Macos(process)
				| crate::expression::Process::Arm64Linux(process)
				| crate::expression::Process::Arm64Macos(process) => {
					// Ensure the command expression is present.
					let hash = process.command;
					let exists = self.expression_exists(hash).await?;
					if !exists {
						missing.push(hash);
					}

					// Ensure the args expression is present.
					let hash = process.args;
					let exists = self.expression_exists(hash).await?;
					if !exists {
						missing.push(hash);
					}

					// Ensure the env expression is present.
					let hash = process.env;
					let exists = self.expression_exists(hash).await?;
					if !exists {
						missing.push(hash);
					}

					// Ensure the outputs expressions are present.
					missing.extend(
						futures::stream::iter(
							process
								.outputs
								.values()
								.flat_map(|output| output.dependencies.values())
								.copied()
								.map(Ok::<_, anyhow::Error>),
						)
						.try_filter_map(|hash| async move {
							let exists = self.expression_exists(hash).await?;
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

				crate::expression::Process::Js(process) => {
					// Ensure the module is present.
					let hash = process.module;
					let exists = self.expression_exists(hash).await?;
					if !exists {
						missing.push(hash);
					}

					// Ensure all of the args are present.
					missing.extend(
						futures::stream::iter(
							process.args.iter().copied().map(Ok::<_, anyhow::Error>),
						)
						.try_filter_map(|hash| async move {
							let exists = self.expression_exists(hash).await?;
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
			},

			// If this expression is a target, ensure its children are present.
			Expression::Target(target) => {
				// Ensure all of the args are present.
				missing.extend(
					futures::stream::iter(target.args.iter().copied().map(Ok::<_, anyhow::Error>))
						.try_filter_map(|hash| async move {
							let exists = self.expression_exists(hash).await?;
							if exists {
								Ok(None)
							} else {
								Ok(Some(hash))
							}
						})
						.try_collect::<Vec<Hash>>()
						.await?,
				);

				// Ensure the package is present.
				let hash = target.package.hash;
				let exists = self.expression_exists(hash).await?;
				if !exists {
					missing.push(hash);
				}
			},

			// If this expression is an array, ensure the values are present.
			Expression::Array(array) => {
				missing.extend(
					futures::stream::iter(array.iter().copied().map(Ok::<_, anyhow::Error>))
						.try_filter_map(|hash| async move {
							let exists = self.expression_exists(hash).await?;
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
							let exists = self.expression_exists(hash).await?;
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

		// Serialize the expression.
		let data = serde_json::to_vec(&expression)?;
		let hash = Hash::new(&data);

		// Add the expression to the database.
		self.database_transaction(|txn| {
			let sql = r#"
				replace into expressions (
					hash, data
				) values (
					$1, $2
				)
			"#;
			let params = (hash.to_string(), data);
			txn.execute(sql, params)?;
			Ok(())
		})
		.await?;

		Ok(AddExpressionOutcome::Added { hash })
	}

	pub async fn expression_exists(self: &Arc<Self>, hash: Hash) -> Result<bool> {
		self.database_transaction(|txn| Self::expression_exists_with_transaction(txn, hash))
			.await
	}

	pub fn expression_exists_with_transaction(
		txn: &rusqlite::Transaction<'_>,
		hash: Hash,
	) -> Result<bool> {
		let sql = r#"
			select
				count(*) > 0
			from
				expressions
			where
				hash = $1
		"#;
		let params = (hash.to_string(),);
		let exists = txn
			.prepare_cached(sql)
			.context("Failed to prepare the query.")?
			.query(params)
			.context("Failed to execute the query.")?
			.and_then(|row| row.get::<_, bool>(0))
			.next()
			.unwrap()?;
		Ok(exists)
	}

	pub async fn get_expression(self: &Arc<Self>, hash: Hash) -> Result<Expression> {
		let expression = self
			.try_get_expression(hash)
			.await?
			.ok_or_else(|| anyhow!(r#"Failed to find the expression with hash "{hash}"."#))?;
		Ok(expression)
	}

	pub fn get_expression_with_transaction(
		txn: &rusqlite::Transaction,
		hash: Hash,
	) -> Result<Expression> {
		Self::try_get_expression_with_transaction(txn, hash)?
			.ok_or_else(|| anyhow!(r#"Failed to find the expression with hash "{}"."#, hash))
	}

	pub async fn try_get_expression(self: &Arc<Self>, hash: Hash) -> Result<Option<Expression>> {
		let maybe_expression = self
			.database_transaction(|txn| Self::try_get_expression_with_transaction(txn, hash))
			.await?;
		Ok(maybe_expression)
	}

	pub fn try_get_expression_with_transaction(
		txn: &rusqlite::Transaction,
		hash: Hash,
	) -> Result<Option<Expression>> {
		let sql = r#"
			select
				data
			from
				expressions
			where
				hash = $1
		"#;
		let params = (hash.to_string(),);
		let mut statement = txn
			.prepare_cached(sql)
			.context("Failed to prepare the query.")?;
		let maybe_expression = statement
			.query(params)
			.context("Failed to execute the query.")?
			.and_then(|row| {
				let data = row.get::<_, Vec<u8>>(0)?;
				let expression = serde_json::from_slice(&data)?;
				Ok::<_, anyhow::Error>(expression)
			})
			.next()
			.transpose()?;
		Ok(maybe_expression)
	}

	pub async fn get_expression_with_output(
		self: &Arc<Self>,
		hash: Hash,
	) -> Result<(Expression, Option<Hash>)> {
		self.try_get_expression_with_output(hash)
			.await?
			.ok_or_else(|| anyhow!(r#"Failed to find the expression with hash "{}"."#, hash))
	}

	pub fn get_expression_with_output_with_transaction(
		txn: &rusqlite::Transaction,
		hash: Hash,
	) -> Result<(Expression, Option<Hash>)> {
		Self::try_get_expression_with_output_with_transaction(txn, hash)?
			.ok_or_else(|| anyhow!(r#"Failed to find the expression with hash "{}"."#, hash))
	}

	pub async fn try_get_expression_with_output(
		self: &Arc<Self>,
		hash: Hash,
	) -> Result<Option<(Expression, Option<Hash>)>> {
		let maybe = self
			.database_transaction(|txn| {
				Self::try_get_expression_with_output_with_transaction(txn, hash)
			})
			.await?;
		Ok(maybe)
	}

	pub fn try_get_expression_with_output_with_transaction(
		txn: &rusqlite::Transaction,
		hash: Hash,
	) -> Result<Option<(Expression, Option<Hash>)>> {
		let sql = r#"
			select
				data,
				output_hash
			from
				expressions
			where
				hash = $1
		"#;
		let params = (hash.to_string(),);
		let mut statement = txn
			.prepare_cached(sql)
			.context("Failed to prepare the query.")?;
		let maybe_expression_with_output = statement
			.query(params)
			.context("Failed to execute the query.")?
			.and_then(|row| {
				let data = row.get::<_, Vec<u8>>(0)?;
				let output_hash = row.get::<_, Option<String>>(1)?;
				let output_hash = if let Some(output_hash) = output_hash {
					Some(output_hash.parse()?)
				} else {
					None
				};
				let expression = serde_json::from_slice(&data)?;
				Ok::<_, anyhow::Error>((expression, output_hash))
			})
			.next()
			.transpose()?;
		Ok(maybe_expression_with_output)
	}

	pub async fn delete_expression(self: &Arc<Self>, hash: Hash) -> Result<()> {
		self.database_transaction(|txn| Self::delete_expression_with_transaction(txn, hash))
			.await
	}

	pub fn delete_expression_with_transaction(
		txn: &rusqlite::Transaction,
		hash: Hash,
	) -> Result<()> {
		let sql = r#"
			delete from expressions where hash = $1
		"#;
		let params = (hash.to_string(),);
		let mut statement = txn
			.prepare_cached(sql)
			.context("Failed to prepare the query.")?;
		statement
			.execute(params)
			.context("Failed to execute the query.")?;
		Ok(())
	}

	pub async fn add_evaluation(
		self: &Arc<Self>,
		parent_hash: Hash,
		child_hash: Hash,
	) -> Result<()> {
		self.database_transaction(|txn| {
			Self::add_evaluation_with_transaction(txn, parent_hash, child_hash)
		})
		.await
	}

	pub fn add_evaluation_with_transaction(
		txn: &rusqlite::Transaction,
		parent_hash: Hash,
		child_hash: Hash,
	) -> Result<()> {
		let sql = r#"
				replace into evaluations (
					parent_hash, child_hash
				) values (
					$1, $2
				)
			"#;
		let params = (parent_hash.to_string(), child_hash.to_string());
		txn.execute(sql, params)?;
		Ok(())
	}

	pub fn get_evaluations_with_transaction<'a>(
		txn: &'a rusqlite::Transaction<'a>,
		hash: Hash,
	) -> Result<Vec<Hash>> {
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
			.query_and_then(params, |row| {
				let hash = row.get::<_, String>(0)?;
				let hash = hash.parse().with_context(|| "Failed to parse the hash.")?;
				Ok::<_, anyhow::Error>(hash)
			})
			.context("Failed to execute the query.")?
			.collect::<Result<_>>()?;
		Ok(evaluations)
	}
}

pub type AddExpressionRequest = Expression;

pub type AddExpressionResponse = AddExpressionOutcome;

impl Server {
	pub(super) async fn handle_create_expression_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let hash = if let ["expressions", hash] = path_components.as_slice() {
			hash
		} else {
			bail!("Unexpected path.");
		};
		let _hash: Hash = match hash.parse() {
			Ok(hash) => hash,
			Err(_) => return Ok(bad_request()),
		};

		// Read and deserialize the request body.
		let body = hyper::body::to_bytes(request.into_body())
			.await
			.context("Failed to read the request body.")?;
		let expression =
			serde_json::from_slice(&body).context("Failed to deserialize the request body.")?;

		// Add the expression.
		let outcome = self
			.try_add_expression(&expression)
			.await
			.context("Failed to get the expression.")?;

		// Create the response.
		let body =
			serde_json::to_vec(&outcome).context("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}

impl Server {
	pub(super) async fn handle_get_expression_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let hash = if let ["expressions", hash] = path_components.as_slice() {
			hash
		} else {
			bail!("Unexpected path.");
		};
		let hash = match hash.parse() {
			Ok(hash) => hash,
			Err(_) => return Ok(bad_request()),
		};

		// Get the expression.
		let expression = self
			.database_transaction(|txn| {
				let expression = Self::try_get_expression_with_transaction(txn, hash)
					.context("Failed to get the expression.")?;
				Ok(expression)
			})
			.await?;

		// Create the response.
		let body =
			serde_json::to_vec(&expression).context("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}

impl Server {
	/// Retrieve the memoized output from a previous evaluation of an expression, if one exists, either on this server or one of its peer servers.
	#[async_recursion]
	#[must_use]
	pub async fn get_memoized_evaluation(
		self: &Arc<Self>,
		expression_hash: Hash,
	) -> Result<Option<Expression>> {
		// Check if we have memoized a previous evaluation of the expression.
		if let Some(output) = self.get_local_memoized_evaluation(&expression_hash).await? {
			return Ok(Some(output));
		}

		// Otherwise, check if any of our peers have memoized a previous evaluation of the expression.
		let peer_futures = self
			.peers
			.iter()
			.map(|peer| peer.get_memoized_evaluation(expression_hash).boxed());
		if let Ok((Some(output), _)) = select_ok(peer_futures).await {
			return Ok(Some(output));
		}

		// Otherwise, there is no memoized evaluation of the expression.
		Ok(None)
	}

	/// Memoize the output from the evaluation of an expression.
	pub async fn set_expression_output(
		self: &Arc<Self>,
		hash: Hash,
		output_hash: Hash,
	) -> Result<()> {
		self.database_transaction(|txn| {
			let sql = r#"
				update
					expressions
				set
					output_hash = $2
				where
					hash = $1
			"#;
			let params = (hash.to_string(), output_hash.to_string());
			txn.execute(sql, params)
				.context("Failed to execute the query.")?;
			Ok(())
		})
		.await?;
		Ok(())
	}

	/// Retrieve the memoized output from a previous evaluation of an expression, if one exists on this server.
	pub async fn get_local_memoized_evaluation(
		self: &Arc<Self>,
		expression_hash: &Hash,
	) -> Result<Option<Expression>> {
		// Retrieve a previous evaluation of the expression from the database.
		let output = self
			.database_transaction(|txn| {
				let sql = r#"
					select
						output
					from
						evaluations
					where
						expression_hash = $1
				"#;
				let params = (expression_hash.to_string(),);
				let mut statement = txn
					.prepare_cached(sql)
					.context("Failed to prepare the query.")?;
				let expression: Option<Vec<u8>> = statement
					.query(params)
					.context("Failed to execute the query.")?
					.and_then(|row| row.get::<_, Vec<u8>>(0))
					.next()
					.transpose()
					.context("Failed to read a row from the query.")?;
				Ok(expression)
			})
			.await?;

		// Deserialize the expression.
		let output = if let Some(output) = output {
			let output = serde_json::from_slice(&output)?;
			Some(output)
		} else {
			None
		};

		Ok(output)
	}
}
