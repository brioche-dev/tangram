use crate::{
	branch, directory, file, leaf, mutation, object, package, symlink, target, template, Branch,
	Client, Directory, File, Leaf, Mutation, Package, Result, Symlink, Target, Template, WrapErr,
};
use async_compression::futures::write;
use bytes::Bytes;
use derive_more::{From, TryInto, TryUnwrap};
use futures::{stream::FuturesUnordered, TryStreamExt};
use std::collections::BTreeMap;

/// A value.
#[derive(Clone, Debug, From, TryInto, serde::Deserialize, TryUnwrap)]
#[serde(try_from = "Data")]
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
		self.children()
			.into_iter()
			.map(|child| async move { child.store(client).await })
			.collect::<FuturesUnordered<_>>()
			.try_collect()
			.await?;
		let data = self.to_data();
		Ok(data)
	}

	pub fn to_data(&self) -> Data {
		match self {
			Self::Null(()) => Data::Null(()),
			Self::Bool(bool) => Data::Bool(*bool),
			Self::Number(number) => Data::Number(*number),
			Self::String(string) => Data::String(string.clone()),
			Self::Bytes(bytes) => Data::Bytes(bytes.clone()),
			Self::Leaf(leaf) => Data::Leaf(leaf.expect_id().clone()),
			Self::Branch(branch) => Data::Branch(branch.expect_id().clone()),
			Self::Directory(directory) => Data::Directory(directory.expect_id().clone()),
			Self::File(file) => Data::File(file.expect_id().clone()),
			Self::Mutation(mutation) => Data::Mutation(mutation.to_data().clone()),
			Self::Symlink(symlink) => Data::Symlink(symlink.expect_id().clone()),
			Self::Template(template) => Data::Template(template.to_data().clone()),
			Self::Package(package) => Data::Package(package.expect_id().clone()),
			Self::Target(target) => Data::Target(target.expect_id().clone()),
			Self::Array(array) => Data::Array(array.iter().map(Value::to_data).collect()),
			Self::Map(map) => Data::Map(
				map.iter()
					.map(|(key, value)| (key.clone(), value.to_data()))
					.collect(),
			),
		}
	}

	pub fn from_data(data: Data) -> Self {
		match data {
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
				Self::Array(array.into_iter().map(Value::from_data).collect::<Vec<_>>())
			},
			Data::Map(map) => Self::Map(
				map.into_iter()
					.map(|(key, value)| (key, Value::from_data(value)))
					.collect(),
			),
		}
	}

	pub fn children(&self) -> Vec<object::Handle> {
		match self {
			Self::Null(()) | Self::Bool(_) | Self::Number(_) | Self::String(_) | Self::Bytes(_) => {
				vec![]
			},
			Self::Leaf(leaf) => vec![leaf.handle().clone()],
			Self::Branch(branch) => vec![branch.handle().clone()],
			Self::Directory(directory) => vec![directory.handle().clone()],
			Self::File(file) => vec![file.handle().clone()],
			Self::Symlink(symlink) => vec![symlink.handle().clone()],
			Self::Template(template) => template.children(),
			Self::Mutation(mutation) => mutation.children(),
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
			Self::Null(()) | Self::Bool(_) | Self::Number(_) | Self::String(_) | Self::Bytes(_) => {
				vec![]
			},
			Self::Leaf(id) => vec![id.clone().into()],
			Self::Branch(id) => vec![id.clone().into()],
			Self::Directory(id) => vec![id.clone().into()],
			Self::File(id) => vec![id.clone().into()],
			Self::Symlink(id) => vec![id.clone().into()],
			Self::Template(template) => template.children(),
			Self::Mutation(mutation) => mutation.children(),
			Self::Package(id) => vec![id.clone().into()],
			Self::Target(id) => vec![id.clone().into()],
			Self::Array(array) => array.iter().flat_map(Self::children).collect(),
			Self::Map(map) => map.values().flat_map(Self::children).collect(),
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
				write!(f, "{leaf}")?;
			},
			Value::Branch(branch) => {
				write!(f, "{branch}")?;
			},
			Value::Directory(directory) => {
				write!(f, "{directory}")?;
			},
			Value::File(file) => {
				write!(f, "{file}")?;
			},
			Value::Symlink(symlink) => {
				write!(f, "{symlink}")?;
			},
			Value::Template(template) => {
				write!(f, "{template}")?;
			},
			Value::Mutation(mutation) => {
				write!(f, "{mutation}")?;
			},
			Value::Package(package) => {
				write!(f, "{package}")?;
			},
			Value::Target(target) => {
				write!(f, "{target}")?;
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
				write!(f, "{{")?;
				if !map.is_empty() {
					write!(f, " ")?;
				}
				for (i, (key, value)) in map.iter().enumerate() {
					write!(f, "{key}: {value}")?;
					if i < map.len() - 1 {
						write!(f, ", ")?;
					}
				}
				if !map.is_empty() {
					write!(f, " ")?;
				}
				write!(f, "}}")?;
			},
		}
		Ok(())
	}
}

impl From<Data> for Value {
	fn from(value: Data) -> Self {
		Self::from_data(value)
	}
}

// impl TryFrom<Data> for Value {
// 	type Error = Error;

// 	fn try_from(value: Data) -> std::result::Result<Self, Self::Error> {
// 		Ok(Self::from_data(value))
// 	}
// }
