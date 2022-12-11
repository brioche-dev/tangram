use crate::{
	checksum::Checksum, db::ExpressionWithOutput, hash::Hash, system::System, util::path_exists,
	State,
};
use anyhow::{bail, Context, Result};
use byteorder::{ReadBytesExt, WriteBytesExt};
use camino::Utf8PathBuf;
use lmdb::{Cursor, Transaction};
use std::{collections::BTreeMap, sync::Arc};
use url::Url;

#[derive(
	Clone,
	Debug,
	PartialEq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(tag = "type", content = "value")]
pub enum Expression {
	#[buffalo(id = 0)]
	#[serde(rename = "null")]
	Null(()),

	#[buffalo(id = 1)]
	#[serde(rename = "bool")]
	Bool(bool),

	#[buffalo(id = 2)]
	#[serde(rename = "number")]
	Number(f64),

	#[buffalo(id = 3)]
	#[serde(rename = "string")]
	String(Arc<str>),

	#[buffalo(id = 4)]
	#[serde(rename = "directory")]
	Directory(Directory),

	#[buffalo(id = 5)]
	#[serde(rename = "file")]
	File(File),

	#[buffalo(id = 6)]
	#[serde(rename = "symlink")]
	Symlink(Symlink),

	#[buffalo(id = 7)]
	#[serde(rename = "dependency")]
	Dependency(Dependency),

	#[buffalo(id = 8)]
	#[serde(rename = "package")]
	Package(Package),

	#[buffalo(id = 9)]
	#[serde(rename = "template")]
	Template(Template),

	#[buffalo(id = 10)]
	#[serde(rename = "placeholder")]
	Placeholder(Placeholder),

	#[buffalo(id = 11)]
	#[serde(rename = "download")]
	Download(Download),

	#[buffalo(id = 12)]
	#[serde(rename = "process")]
	Process(Process),

	#[buffalo(id = 13)]
	#[serde(rename = "target")]
	Target(Target),

	#[buffalo(id = 14)]
	#[serde(rename = "array")]
	Array(Array),

	#[buffalo(id = 15)]
	#[serde(rename = "map")]
	Map(Map),
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Directory {
	#[buffalo(id = 0)]
	pub entries: BTreeMap<String, Hash>,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct File {
	#[buffalo(id = 0)]
	pub blob: Hash,

	#[buffalo(id = 1)]
	pub executable: bool,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Symlink {
	#[buffalo(id = 0)]
	pub target: Utf8PathBuf,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Dependency {
	#[buffalo(id = 0)]
	pub artifact: Hash,

	#[buffalo(id = 1)]
	pub path: Option<Utf8PathBuf>,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Package {
	#[buffalo(id = 0)]
	pub source: Hash,

	#[buffalo(id = 1)]
	pub dependencies: BTreeMap<Arc<str>, Hash>,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Template {
	#[buffalo(id = 0)]
	pub components: Vec<Hash>,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Placeholder {
	#[buffalo(id = 0)]
	pub name: String,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Download {
	#[buffalo(id = 0)]
	pub url: Url,

	#[buffalo(id = 1)]
	pub checksum: Option<Checksum>,

	#[buffalo(id = 2)]
	pub unpack: bool,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Process {
	#[buffalo(id = 0)]
	pub system: System,

	#[buffalo(id = 1)]
	pub working_directory: Hash,

	#[buffalo(id = 2)]
	pub env: Hash,

	#[buffalo(id = 3)]
	pub command: Hash,

	#[buffalo(id = 4)]
	pub args: Hash,

	#[buffalo(id = 5)]
	#[serde(default)]
	pub network: bool,

	#[buffalo(id = 6)]
	#[serde(default)]
	pub checksum: Option<Checksum>,

	#[buffalo(id = 7)]
	#[serde(default, rename = "unsafe")]
	pub is_unsafe: bool,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Target {
	#[buffalo(id = 0)]
	pub package: Hash,

	#[buffalo(id = 1)]
	pub name: String,

	#[buffalo(id = 2)]
	pub args: Hash,
}

pub type Array = Vec<Hash>;

pub type Map = BTreeMap<Arc<str>, Hash>;

impl Expression {
	pub fn deserialize<R>(mut reader: R) -> Result<Expression>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			bail!(r#"Cannot deserialize expression with version "{version}"."#);
		}

		// Deserialize the expression.
		let expression = buffalo::from_reader(reader)?;

		Ok(expression)
	}

	pub fn deserialize_from_slice(slice: &[u8]) -> Result<Expression> {
		Expression::deserialize(slice)
	}

	pub fn serialize<W>(&self, mut writer: W) -> Result<()>
	where
		W: std::io::Write,
	{
		// Write the version.
		writer.write_u8(0)?;

		// Write the expression.
		buffalo::to_writer(self, &mut writer)?;

		Ok(())
	}

	#[must_use]
	pub fn serialize_to_vec(&self) -> Vec<u8> {
		let mut data = Vec::new();
		self.serialize(&mut data).unwrap();
		data
	}

	#[must_use]
	pub fn serialize_to_vec_and_hash(&self) -> (Hash, Vec<u8>) {
		let data = self.serialize_to_vec();
		let hash = Hash::new(&data);
		(hash, data)
	}

	#[must_use]
	pub fn hash(&self) -> Hash {
		let data = self.serialize_to_vec();
		Hash::new(&data)
	}
}

impl Expression {
	#[must_use]
	pub fn as_null(&self) -> Option<&()> {
		if let Expression::Null(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_bool(&self) -> Option<&bool> {
		if let Expression::Bool(v) = self {
			Some(v)
		} else {
			None
		}
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
	pub fn as_package(&self) -> Option<&Package> {
		if let Expression::Package(v) = self {
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
	pub fn as_placeholder(&self) -> Option<&Placeholder> {
		if let Expression::Placeholder(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_download(&self) -> Option<&Download> {
		if let Expression::Download(v) = self {
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
	pub fn into_null(self) -> Option<()> {
		if let Expression::Null(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_bool(self) -> Option<bool> {
		if let Expression::Bool(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_number(self) -> Option<f64> {
		if let Expression::Number(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_string(self) -> Option<Arc<str>> {
		if let Expression::String(v) = self {
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
	pub fn into_package(self) -> Option<Package> {
		if let Expression::Package(v) = self {
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
	pub fn into_placeholder(self) -> Option<Placeholder> {
		if let Expression::Placeholder(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_download(self) -> Option<Download> {
		if let Expression::Download(v) = self {
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AddExpressionOutcome {
	Added { hash: Hash },
	DirectoryMissingEntries { entries: Vec<(String, Hash)> },
	FileMissingBlob { blob_hash: Hash },
	DependencyMissing { hash: Hash },
	MissingExpressions { hashes: Vec<Hash> },
}

impl State {
	pub async fn add_expression(&self, expression: &Expression) -> Result<Hash> {
		match self.try_add_expression(expression).await? {
			AddExpressionOutcome::Added { hash } => Ok(hash),
			_ => bail!("Failed to add the expression."),
		}
	}

	// Add an expression after ensuring all its references are present.
	#[allow(clippy::too_many_lines, clippy::match_same_arms)]
	pub async fn try_add_expression(
		&self,
		expression: &Expression,
	) -> Result<AddExpressionOutcome> {
		// Before adding this expression, we need to ensure all its references are present.
		let mut missing = Vec::new();
		match expression {
			// If this expression is null, there is nothing to ensure.
			Expression::Null(_) => {},

			// If this expression is bool, there is nothing to ensure.
			Expression::Bool(_) => {},

			// If this expression is number, there is nothing to ensure.
			Expression::Number(_) => {},

			// If this expression is string, there is nothing to ensure.
			Expression::String(_) => {},

			// If this expression is a directory, ensure all its entry expressions are present.
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

			// If this expression is a dependency, ensure the dependency expression is present.
			Expression::Dependency(dependency) => {
				let hash = dependency.artifact;
				let exists = self.expression_exists_local(hash)?;
				if !exists {
					return Ok(AddExpressionOutcome::DependencyMissing { hash });
				}
			},

			// If this expression is a package, ensure its source and dependencies expressions are present.
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

			// If this expression is a template, ensure the components are present.
			Expression::Template(template) => {
				for hash in template.components.iter().copied() {
					if !self.expression_exists_local(hash)? {
						missing.push(hash);
					}
				}
			},

			// If this expression is a placeholder, there is nothing to ensure.
			Expression::Placeholder(_) => {},

			// If this expression is a download, there is nothing to ensure.
			Expression::Download(_) => {},

			// If this expression is a process, ensure its children are present.
			Expression::Process(process) => {
				// Ensure the command expression is present.
				let hash = process.working_directory;
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
			},

			// If this expression is a target, ensure its children are present.
			Expression::Target(target) => {
				// Ensure the package expression is present.
				let hash = target.package;
				let exists = self.expression_exists_local(hash)?;
				if !exists {
					missing.push(hash);
				}

				// Ensure the args expression is present.
				let hash = target.args;
				let exists = self.expression_exists_local(hash)?;
				if !exists {
					missing.push(hash);
				}
			},

			// If this expression is an array, ensure the values expressions are present.
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
				.iter_dup_of(hash.as_slice())
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
