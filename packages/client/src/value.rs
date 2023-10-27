use crate::{
	branch, directory, file, leaf, mutation, object, package, symlink, target, template, Branch,
	Client, Directory, File, Leaf, Mutation, Package, Result, Symlink, Target, Template, WrapErr,
};
use bytes::Bytes;
use derive_more::{From, TryInto, TryUnwrap};
use std::collections::BTreeMap;

/// A value.
#[derive(Clone, Debug, From, TryInto, serde::Serialize, serde::Deserialize, TryUnwrap)]
#[serde(into = "Data", try_from = "Data")]
#[try_unwrap(ref)]
pub enum Value {
	/// A null value.
	Null(()),

	/// A bool value.
	Bool(bool),

	/// A number value.
	Number(f64),

	/// A string value.
	String(String),

	/// A bytes value.
	Bytes(Bytes),

	/// A leaf value.
	Leaf(Leaf),

	/// A branch value.
	Branch(Branch),

	/// A directory value.
	Directory(Directory),

	/// A file value.
	File(File),

	/// A symlink value.
	Symlink(Symlink),

	/// A template value.
	Template(Template),

	/// A mutation value.
	Mutation(Mutation),

	/// A package value.
	Package(Package),

	/// A target value.
	Target(Target),

	/// An array value.
	Array(Vec<Value>),

	/// A map value.
	Map(BTreeMap<String, Value>),
}

/// Value data.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum Data {
	Null(()),
	Bool(bool),
	Number(f64),
	String(String),
	Bytes(Bytes),
	Leaf(leaf::Id),
	Branch(branch::Id),
	Directory(directory::Id),
	File(file::Id),
	Symlink(symlink::Id),
	Template(template::Data),
	Mutation(mutation::Data),
	Package(package::Id),
	Target(target::Id),
	Array(Vec<Data>),
	Map(BTreeMap<String, Data>),
}

impl Value {
	pub async fn data(&self, client: &dyn Client) -> Result<Data> {
		match self {
			Value::Directory(directory) => directory.handle().store(client).await?,
			Value::File(file) => file.handle().store(client).await?,
			Value::Symlink(symlink) => symlink.handle().store(client).await?,
			Value::Package(package) => package.handle().store(client).await?,
			Value::Target(target) => target.handle().store(client).await?,
			_ => {},
		}
		Ok(self.clone().into())
	}

	pub fn children(&self) -> Vec<object::Handle> {
		match self {
			Self::Null(())
			| Self::Bool(_)
			| Self::Number(_)
			| Self::String(_)
			| Self::Bytes(_)
			| Self::Mutation(_) => {
				vec![]
			},
			Self::Leaf(leaf) => vec![leaf.handle().clone()],
			Self::Branch(branch) => vec![branch.handle().clone()],
			Self::Directory(directory) => vec![directory.handle().clone()],
			Self::File(file) => vec![file.handle().clone()],
			Self::Symlink(symlink) => vec![symlink.handle().clone()],
			Self::Template(template) => template.children(),
			Self::Package(package) => vec![package.handle().clone()],
			Self::Target(target) => vec![target.handle().clone()],
			Self::Array(array) => array.iter().flat_map(Self::children).collect(),
			Self::Map(map) => map.values().flat_map(Self::children).collect(),
		}
	}
}

impl Data {
	pub fn serialize(&self) -> Result<Bytes> {
		serde_json::to_vec(self)
			.map(Into::into)
			.wrap_err("Failed to serialize the data.")
	}

	pub fn deserialize(bytes: &Bytes) -> Result<Self> {
		serde_json::from_reader(bytes.as_ref()).wrap_err("Failed to deserialize the data.")
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		match self {
			Self::Null(())
			| Self::Bool(_)
			| Self::Number(_)
			| Self::String(_)
			| Self::Bytes(_)
			| Self::Mutation(_) => {
				vec![]
			},
			Self::Leaf(id) => vec![id.clone().into()],
			Self::Branch(id) => vec![id.clone().into()],
			Self::Directory(id) => vec![id.clone().into()],
			Self::File(id) => vec![id.clone().into()],
			Self::Symlink(id) => vec![id.clone().into()],
			Self::Template(template) => template.children(),
			Self::Package(id) => vec![id.clone().into()],
			Self::Target(id) => vec![id.clone().into()],
			Self::Array(array) => array.iter().flat_map(Self::children).collect(),
			Self::Map(map) => map.values().flat_map(Self::children).collect(),
		}
	}
}

impl From<Value> for Data {
	fn from(value: Value) -> Self {
		match value {
			Value::Null(()) => Self::Null(()),
			Value::Bool(bool) => Self::Bool(bool),
			Value::Number(number) => Self::Number(number),
			Value::String(string) => Self::String(string.clone()),
			Value::Bytes(bytes) => Self::Bytes(bytes.clone()),
			Value::Leaf(leaf) => Self::Leaf(leaf.expect_id().clone()),
			Value::Branch(branch) => Self::Branch(branch.expect_id().clone()),
			Value::Directory(directory) => Self::Directory(directory.expect_id().clone()),
			Value::File(file) => Self::File(file.expect_id().clone()),
			Value::Mutation(mutation) => Self::Mutation(mutation.to_data().clone()),
			Value::Symlink(symlink) => Self::Symlink(symlink.expect_id().clone()),
			Value::Template(template) => Self::Template(template.to_data().clone()),
			Value::Package(package) => Self::Package(package.expect_id().clone()),
			Value::Target(target) => Self::Target(target.expect_id().clone()),
			Value::Array(array) => Self::Array(array.into_iter().map(Into::into).collect()),
			Value::Map(map) => Self::Map(
				map.into_iter()
					.map(|(key, value)| (key.clone(), value.into()))
					.collect(),
			),
		}
	}
}

impl From<Data> for Value {
	fn from(value: Data) -> Self {
		match value {
			Data::Null(()) => Self::Null(()),
			Data::Bool(bool) => Self::Bool(bool),
			Data::Number(number) => Self::Number(number),
			Data::String(string) => Self::String(string),
			Data::Bytes(bytes) => Self::Bytes(bytes),
			Data::Leaf(id) => Self::Leaf(Leaf::with_id(id)),
			Data::Branch(id) => Self::Branch(Branch::with_id(id)),
			Data::Directory(id) => Self::Directory(Directory::with_id(id)),
			Data::File(id) => Self::File(File::with_id(id)),
			Data::Mutation(mutation) => Self::Mutation(Mutation::from_data(mutation)),
			Data::Symlink(id) => Self::Symlink(Symlink::with_id(id)),
			Data::Template(template) => Self::Template(Template::from_data(template)),
			Data::Package(id) => Self::Package(Package::with_id(id)),
			Data::Target(id) => Self::Target(Target::with_id(id)),
			Data::Array(array) => {
				Self::Array(array.into_iter().map(Into::into).collect::<Vec<_>>())
			},
			Data::Map(map) => Self::Map(
				map.into_iter()
					.map(|(key, value)| (key, value.into()))
					.collect(),
			),
		}
	}
}

impl std::fmt::Display for Value {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Value::Null(()) => {
				write!(f, "null")?;
			},
			Value::Bool(bool) => {
				write!(f, "{bool}")?;
			},
			Value::Number(number) => {
				write!(f, "{number}")?;
			},
			Value::String(string) => {
				write!(f, "\"{string}\"")?;
			},
			Value::Bytes(bytes) => {
				write!(f, "{}", hex::encode(bytes))?;
			},
			Value::Leaf(leaf) => {
				write!(f, "{}", leaf.expect_id())?;
			},
			Value::Branch(branch) => {
				write!(f, "{}", branch.expect_id())?;
			},
			Value::Directory(directory) => {
				write!(f, "{}", directory.expect_id())?;
			},
			Value::File(file) => {
				write!(f, "{}", file.expect_id())?;
			},
			Value::Symlink(symlink) => {
				write!(f, "{}", symlink.expect_id())?;
			},
			Value::Template(template) => {
				write!(f, "{template}")?;
			},
			Value::Mutation(mutation) => {
				write!(f, "{mutation}")?;
			},
			Value::Package(package) => {
				write!(f, "{}", package.expect_id())?;
			},
			Value::Target(target) => {
				write!(f, "{}", target.expect_id())?;
			},
			Value::Array(array) => {
				write!(f, "[")?;
				for (i, value) in array.iter().enumerate() {
					write!(f, "{value}")?;
					if i < array.len() - 1 {
						write!(f, ", ")?;
					}
				}
				write!(f, "]")?;
			},
			Value::Map(map) => {
				write!(f, "[")?;
				for (i, (key, value)) in map.iter().enumerate() {
					write!(f, "{key}:{value}")?;
					if i < map.len() - 1 {
						write!(f, ", ")?;
					}
				}
				write!(f, "]")?;
			},
		}
		Ok(())
	}
}
