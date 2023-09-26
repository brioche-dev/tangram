use crate::{
	blob, directory, file, object, package, placeholder, symlink, task, template, Artifact, Blob,
	Bytes, Directory, File, Package, Placeholder, Symlink, Task, Template,
};
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
pub enum Data {
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
	Blob(blob::Id),

	#[tangram_serialize(id = 6)]
	Directory(directory::Id),

	#[tangram_serialize(id = 7)]
	File(file::Id),

	#[tangram_serialize(id = 8)]
	Symlink(symlink::Id),

	#[tangram_serialize(id = 9)]
	Placeholder(placeholder::Data),

	#[tangram_serialize(id = 10)]
	Template(template::Data),

	#[tangram_serialize(id = 11)]
	Package(package::Id),

	#[tangram_serialize(id = 12)]
	Task(task::Id),

	#[tangram_serialize(id = 13)]
	Array(Vec<Data>),

	#[tangram_serialize(id = 14)]
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
			| Self::Placeholder(_) => vec![],
			Self::Blob(blob) => vec![blob.handle().clone()],
			Self::Directory(directory) => vec![directory.handle().clone()],
			Self::File(file) => vec![file.handle().clone()],
			Self::Symlink(symlink) => vec![symlink.handle().clone()],
			Self::Template(template) => template.children(),
			Self::Package(package) => vec![package.handle().clone()],
			Self::Task(task) => vec![task.handle().clone()],
			Self::Array(array) => array.iter().flat_map(Self::children).collect(),
			Self::Map(map) => map.values().flat_map(Self::children).collect(),
		}
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		match self {
			Self::Null(_)
			| Self::Bool(_)
			| Self::Number(_)
			| Self::String(_)
			| Self::Bytes(_)
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

impl Value {
	#[must_use]
	pub fn as_null(&self) -> Option<&()> {
		match self {
			Self::Null(value) => Some(value),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_bool(&self) -> Option<&bool> {
		match self {
			Self::Bool(value) => Some(value),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_number(&self) -> Option<&f64> {
		match self {
			Self::Number(value) => Some(value),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_string(&self) -> Option<&String> {
		match self {
			Self::String(value) => Some(value),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_bytes(&self) -> Option<&Bytes> {
		match self {
			Self::Bytes(value) => Some(value),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_placeholder(&self) -> Option<&Placeholder> {
		match self {
			Self::Placeholder(value) => Some(value),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_blob(&self) -> Option<&Blob> {
		match self {
			Self::Blob(value) => Some(value),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_directory(&self) -> Option<&Directory> {
		match self {
			Self::Directory(value) => Some(value),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_file(&self) -> Option<&File> {
		match self {
			Self::File(value) => Some(value),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_symlink(&self) -> Option<&Symlink> {
		match self {
			Self::Symlink(value) => Some(value),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_artifact(&self) -> Option<Artifact> {
		match self {
			Self::Directory(value) => Some(Artifact::Directory(value.clone())),
			Self::File(value) => Some(Artifact::File(value.clone())),
			Self::Symlink(value) => Some(Artifact::Symlink(value.clone())),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_template(&self) -> Option<&Template> {
		match self {
			Self::Template(value) => Some(value),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_package(&self) -> Option<&Package> {
		match self {
			Self::Package(value) => Some(value),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_task(&self) -> Option<&Task> {
		match self {
			Self::Task(value) => Some(value),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_array(&self) -> Option<&Vec<Value>> {
		match self {
			Self::Array(value) => Some(value),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_map(&self) -> Option<&BTreeMap<String, Value>> {
		match self {
			Self::Map(value) => Some(value),
			_ => None,
		}
	}
}
