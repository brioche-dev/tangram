use crate::{builder, hash::Hash, system::System, util::path_exists};
use anyhow::{anyhow, bail, Context, Result};
use async_recursion::async_recursion;
use camino::Utf8PathBuf;
use futures::{
	future::{select_ok, FutureExt},
	stream::TryStreamExt,
};
use std::{collections::BTreeMap, sync::Arc};
use url::Url;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Expression {
	#[serde(rename = "null")]
	Null,
	#[serde(rename = "bool")]
	Bool(bool),
	#[serde(rename = "number")]
	Number(f64),
	#[serde(rename = "string")]
	String(Arc<str>),
	#[serde(rename = "artifact")]
	Artifact(Artifact),
	#[serde(rename = "directory")]
	Directory(Directory),
	#[serde(rename = "file")]
	File(File),
	#[serde(rename = "symlink")]
	Symlink(Symlink),
	#[serde(rename = "dependency")]
	Dependency(Dependency),
	#[serde(rename = "template")]
	Template(Template),
	#[serde(rename = "js")]
	Js(Js),
	#[serde(rename = "fetch")]
	Fetch(Fetch),
	#[serde(rename = "process")]
	Process(Process),
	#[serde(rename = "target")]
	Target(Target),
	#[serde(rename = "array")]
	Array(Array),
	#[serde(rename = "map")]
	Map(Map),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Artifact {
	pub root: Hash,
}

/// An expression representing a directory.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Directory {
	pub entries: BTreeMap<String, Hash>,
}

/// An expression representing a file.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct File {
	pub blob: Hash,
	pub executable: bool,
}

/// An expression representing a symbolic link.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Symlink {
	pub target: Utf8PathBuf,
}

/// An expression representing a dependency on another artifact.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Dependency {
	pub artifact: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Template {
	pub components: Vec<Hash>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Fetch {
	pub url: Url,
	pub hash: Option<Hash>,
	pub unpack: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Process {
	pub system: System,
	pub env: Hash,
	pub command: Hash,
	pub args: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Js {
	pub dependencies: BTreeMap<Arc<str>, Hash>,
	pub artifact: Hash,
	pub path: Option<Utf8PathBuf>,
	pub name: String,
	pub args: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Target {
	pub package: Hash,
	pub name: String,
	pub args: Hash,
}

pub type Array = Vec<Hash>;

pub type Map = BTreeMap<Arc<str>, Hash>;

impl Expression {
	#[must_use]
	pub fn is_null(&self) -> bool {
		matches!(self, Expression::Null)
	}

	#[must_use]
	pub fn as_number(&self) -> Option<&f64> {
		if let Expression::Number(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_string(&self) -> Option<&Arc<str>> {
		if let Expression::String(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_artifact(&self) -> Option<&Artifact> {
		if let Expression::Artifact(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_directory(&self) -> Option<&Directory> {
		if let Expression::Directory(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_file(&self) -> Option<&File> {
		if let Expression::File(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_symlink(&self) -> Option<&Symlink> {
		if let Expression::Symlink(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_dependency(&self) -> Option<&Dependency> {
		if let Expression::Dependency(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_template(&self) -> Option<&Template> {
		if let Expression::Template(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_fetch(&self) -> Option<&Fetch> {
		if let Expression::Fetch(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_process(&self) -> Option<&Process> {
		if let Expression::Process(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_target(&self) -> Option<&Target> {
		if let Expression::Target(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_array(&self) -> Option<&Array> {
		if let Expression::Array(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_map(&self) -> Option<&Map> {
		if let Expression::Map(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

impl Expression {
	#[must_use]
	pub fn into_artifact(self) -> Option<Artifact> {
		if let Expression::Artifact(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_directory(self) -> Option<Directory> {
		if let Expression::Directory(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_file(self) -> Option<File> {
		if let Expression::File(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_symlink(self) -> Option<Symlink> {
		if let Expression::Symlink(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_dependency(self) -> Option<Dependency> {
		if let Expression::Dependency(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_template(self) -> Option<Template> {
		if let Expression::Template(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_fetch(self) -> Option<Fetch> {
		if let Expression::Fetch(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_process(self) -> Option<Process> {
		if let Expression::Process(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_target(self) -> Option<Target> {
		if let Expression::Target(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_array(self) -> Option<Array> {
		if let Expression::Array(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_map(self) -> Option<Map> {
		if let Expression::Map(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AddExpressionOutcome {
	Added { hash: Hash },
	DirectoryMissingEntries { entries: Vec<(String, Hash)> },
	FileMissingBlob { blob_hash: Hash },
	DependencyMissing { hash: Hash },
	MissingExpressions { hashes: Vec<Hash> },
}

impl builder::Shared {
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

			Expression::Js(js) => {
				// Ensure the artifact is present.
				let hash = js.artifact;
				let exists = self.expression_exists(hash).await?;
				if !exists {
					missing.push(hash);
				}

				// Ensure the args are present.
				let hash = js.args;
				let exists = self.expression_exists(hash).await?;
				if !exists {
					missing.push(hash);
				}
			},

			// If this expression is a process, ensure its children are present.
			Expression::Process(process) => {
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
			},

			// If this expression is a target, ensure its children are present.
			Expression::Target(target) => {
				// Ensure the package is present.
				let hash = target.package;
				let exists = self.expression_exists(hash).await?;
				if !exists {
					missing.push(hash);
				}

				// Ensure the args are present.
				let hash = target.args;
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

		// Serialize and hash the expression.
		let data = serde_json::to_vec(&expression)?;
		let hash = Hash::new(&data);

		// Add the expression to the database.
		self.database_transaction(|txn| {
			let sql = r#"
				replace into expressions (
					hash, data
				) values (
					?1, ?2
				)
			"#;
			let params = (hash.to_string(), data);
			txn.execute(sql, params)?;
			Ok(())
		})
		.await?;

		Ok(AddExpressionOutcome::Added { hash })
	}

	pub async fn expression_exists(&self, hash: Hash) -> Result<bool> {
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
				hash = ?1
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

	pub async fn get_expression(&self, hash: Hash) -> Result<Expression> {
		let expression = self
			.try_get_expression(hash)
			.await?
			.ok_or_else(|| anyhow!(r#"Failed to find the expression with hash "{hash}"."#))?;
		Ok(expression)
	}

	pub fn get_expression_with_transaction(
		&self,
		txn: &rusqlite::Transaction,
		hash: Hash,
	) -> Result<Expression> {
		self.try_get_expression_with_transaction(txn, hash)?
			.ok_or_else(|| anyhow!(r#"Failed to find the expression with hash "{}"."#, hash))
	}

	pub async fn try_get_expression(&self, hash: Hash) -> Result<Option<Expression>> {
		let maybe_expression = self
			.database_transaction(|txn| self.try_get_expression_with_transaction(txn, hash))
			.await?;
		Ok(maybe_expression)
	}

	pub fn try_get_expression_with_transaction(
		&self,
		txn: &rusqlite::Transaction,
		hash: Hash,
	) -> Result<Option<Expression>> {
		let sql = r#"
			select
				data
			from
				expressions
			where
				hash = ?1
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
		&self,
		hash: Hash,
	) -> Result<(Expression, Option<Hash>)> {
		self.try_get_expression_with_output(hash)
			.await?
			.ok_or_else(|| anyhow!(r#"Failed to find the expression with hash "{}"."#, hash))
	}

	pub fn get_expression_with_output_with_transaction(
		&self,
		txn: &rusqlite::Transaction,
		hash: Hash,
	) -> Result<(Expression, Option<Hash>)> {
		self.try_get_expression_with_output_with_transaction(txn, hash)?
			.ok_or_else(|| anyhow!(r#"Failed to find the expression with hash "{}"."#, hash))
	}

	pub async fn try_get_expression_with_output(
		&self,
		hash: Hash,
	) -> Result<Option<(Expression, Option<Hash>)>> {
		let maybe = self
			.database_transaction(|txn| {
				self.try_get_expression_with_output_with_transaction(txn, hash)
			})
			.await?;
		Ok(maybe)
	}

	pub fn try_get_expression_with_output_with_transaction(
		&self,
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
				hash = ?1
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
					let output_hash = output_hash
						.parse()
						.context("Failed to parse the output hash.")?;
					Some(output_hash)
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

	pub async fn delete_expression(&self, hash: Hash) -> Result<()> {
		self.database_transaction(|txn| self.delete_expression_with_transaction(txn, hash))
			.await
	}

	pub fn delete_expression_with_transaction(
		&self,
		txn: &rusqlite::Transaction,
		hash: Hash,
	) -> Result<()> {
		let sql = r#"
			delete from expressions where hash = ?1
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

	pub async fn add_evaluation(&self, parent_hash: Hash, child_hash: Hash) -> Result<()> {
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
					?1, ?2
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
				parent_hash = ?1
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

	/// Retrieve the memoized output from a previous evaluation of an expression, if one exists, either on this server or one of its peer servers.
	#[async_recursion]
	#[must_use]
	pub async fn get_memoized_evaluation(
		&self,
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
	pub async fn set_expression_output(&self, hash: Hash, output_hash: Hash) -> Result<()> {
		self.database_transaction(|txn| {
			let sql = r#"
				update
					expressions
				set
					output_hash = ?2
				where
					hash = ?1
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
		&self,
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
						expression_hash = ?1
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
