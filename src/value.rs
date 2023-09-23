use crate::{
	blob, directory, file, object, package, placeholder, return_error, symlink, task, template,
	Blob, Bytes, Directory, File, Package, Placeholder, Relpath, Result, Subpath, Symlink, Task,
	Template,
};
use byteorder::{ReadBytesExt, WriteBytesExt};
use derive_more::{From, TryInto};
use std::collections::BTreeMap;

/// A value.
#[derive(Clone, Debug, From, TryInto)]
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

	/// A relpath value.
	Relpath(Relpath),

	/// A subpath value.
	Subpath(Subpath),

	/// A blob value.
	Blob(Blob),

	/// A directory value.
	Directory(Directory),

	/// A file value.
	File(File),

	/// A symlink value.
	Symlink(Symlink),

	/// A placeholder value.
	Placeholder(Placeholder),

	/// A template value.
	Template(Template),

	/// A package value.
	Package(Package),

	/// A task value.
	Task(Task),

	/// An array value.
	Array(Vec<Value>),

	/// A map value.
	Map(BTreeMap<String, Value>),
}

/// Value data.
#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub(crate) enum Data {
	#[tangram_serialize(id = 0)]
	Null(()),

	#[tangram_serialize(id = 1)]
	Bool(bool),

	#[tangram_serialize(id = 2)]
	Number(f64),

	#[tangram_serialize(id = 3)]
	String(String),

	#[tangram_serialize(id = 4)]
	Bytes(Bytes),

	#[tangram_serialize(id = 5)]
	Relpath(Relpath),

	#[tangram_serialize(id = 6)]
	Subpath(Subpath),

	#[tangram_serialize(id = 7)]
	Blob(blob::Id),

	/// A directory value.
	#[tangram_serialize(id = 8)]
	Directory(directory::Id),

	/// A file value.
	#[tangram_serialize(id = 9)]
	File(file::Id),

	/// A symlink value.
	#[tangram_serialize(id = 10)]
	Symlink(symlink::Id),

	/// A placeholder value.
	#[tangram_serialize(id = 11)]
	Placeholder(placeholder::Data),

	/// A template value.
	#[tangram_serialize(id = 12)]
	Template(template::Data),

	/// A package value.
	#[tangram_serialize(id = 13)]
	Package(package::Id),

	/// A task value.
	#[tangram_serialize(id = 14)]
	Task(task::Id),

	/// An array value.
	#[tangram_serialize(id = 15)]
	Array(Vec<Data>),

	/// A map value.
	#[tangram_serialize(id = 16)]
	Map(BTreeMap<String, Data>),
}

impl Value {
	#[must_use]
	pub(crate) fn to_data(&self) -> Data {
		match self {
			Value::Null(_) => Data::Null(()),
			Value::Bool(value) => Data::Bool(*value),
			Value::Number(value) => Data::Number(*value),
			Value::String(value) => Data::String(value.clone()),
			Value::Bytes(value) => Data::Bytes(value.clone()),
			Value::Relpath(value) => Data::Relpath(value.clone()),
			Value::Subpath(value) => Data::Subpath(value.clone()),
			Value::Blob(value) => Data::Blob(value.expect_id()),
			Value::Directory(value) => Data::Directory(value.expect_id()),
			Value::File(value) => Data::File(value.expect_id()),
			Value::Symlink(value) => Data::Symlink(value.expect_id()),
			Value::Placeholder(value) => Data::Placeholder(value.to_data()),
			Value::Template(value) => Data::Template(value.to_data()),
			Value::Package(value) => Data::Package(value.expect_id()),
			Value::Task(value) => Data::Task(value.expect_id()),
			Value::Array(value) => Data::Array(value.iter().map(Value::to_data).collect()),
			Value::Map(value) => Data::Map(
				value
					.iter()
					.map(|(key, value)| (key.clone(), value.to_data()))
					.collect(),
			),
		}
	}

	#[must_use]
	pub(crate) fn from_data(data: Data) -> Self {
		match data {
			Data::Null(_) => Value::Null(()),
			Data::Bool(bool) => Value::Bool(bool),
			Data::Number(number) => Value::Number(number),
			Data::String(string) => Value::String(string),
			Data::Bytes(bytes) => Value::Bytes(bytes),
			Data::Relpath(relpath) => Value::Relpath(relpath),
			Data::Subpath(subpath) => Value::Subpath(subpath),
			Data::Blob(id) => Value::Blob(Blob::with_id(id)),
			Data::Directory(id) => Value::Directory(Directory::with_id(id)),
			Data::File(id) => Value::File(File::with_id(id)),
			Data::Symlink(id) => Value::Symlink(Symlink::with_id(id)),
			Data::Placeholder(placeholder) => {
				Value::Placeholder(Placeholder::from_data(placeholder))
			},
			Data::Template(template) => Value::Template(Template::from_data(template)),
			Data::Package(id) => Value::Package(Package::with_id(id)),
			Data::Task(id) => Value::Task(Task::with_id(id)),
			Data::Array(data) => {
				Value::Array(data.into_iter().map(Value::from_data).collect::<Vec<_>>())
			},
			Data::Map(data) => Value::Map(
				data.into_iter()
					.map(|(key, value)| (key, Value::from_data(value)))
					.collect(),
			),
		}
	}

	pub fn children(&self) -> Vec<object::Handle> {
		match self {
			Self::Null(_)
			| Self::Bool(_)
			| Self::Number(_)
			| Self::String(_)
			| Self::Bytes(_)
			| Self::Relpath(_)
			| Self::Subpath(_)
			| Self::Placeholder(_) => vec![],
			Self::Blob(blob) => vec![blob.clone().into()],
			Self::Directory(directory) => vec![directory.clone().into()],
			Self::File(file) => vec![file.clone().into()],
			Self::Symlink(symlink) => vec![symlink.clone().into()],
			Self::Template(template) => template.children(),
			Self::Package(package) => vec![package.clone().into()],
			Self::Task(task) => vec![task.clone().into()],
			Self::Array(array) => array.iter().flat_map(Self::children).collect(),
			Self::Map(map) => map.values().flat_map(Self::children).collect(),
		}
	}
}

impl Data {
	pub(crate) fn serialize(&self) -> Result<Vec<u8>> {
		let mut bytes = Vec::new();
		bytes.write_u8(0)?;
		tangram_serialize::to_writer(self, &mut bytes)?;
		Ok(bytes)
	}

	pub(crate) fn deserialize(mut bytes: &[u8]) -> Result<Self> {
		let version = bytes.read_u8()?;
		if version != 0 {
			return_error!(r#"Cannot deserialize a value with version "{version}"."#);
		}
		let value = tangram_serialize::from_reader(bytes)?;
		Ok(value)
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		match self {
			Self::Null(_)
			| Self::Bool(_)
			| Self::Number(_)
			| Self::String(_)
			| Self::Bytes(_)
			| Self::Relpath(_)
			| Self::Subpath(_)
			| Self::Placeholder(_) => vec![],
			Self::Blob(id) => vec![(*id).into()],
			Self::Directory(id) => vec![(*id).into()],
			Self::File(id) => vec![(*id).into()],
			Self::Symlink(id) => vec![(*id).into()],
			Self::Template(template) => template.children(),
			Self::Package(id) => vec![(*id).into()],
			Self::Task(id) => vec![(*id).into()],
			Self::Array(array) => array.iter().flat_map(Self::children).collect(),
			Self::Map(map) => map.values().flat_map(Self::children).collect(),
		}
	}
}
