use crate::{
	any, array, blob, bool, bytes, directory, file, null, number, object, package, placeholder,
	relpath, resource, return_error, string, subpath, symlink, target, task, template, Id, Result,
};
use byteorder::{ReadBytesExt, WriteBytesExt};

/// A value.
#[derive(Clone, Debug)]
pub enum Value {
	/// A null value.
	Null(null::Value),

	/// A bool value.
	Bool(bool::Value),

	/// A number value.
	Number(number::Value),

	/// A string value.
	String(string::Value),

	/// A bytes value.
	Bytes(bytes::Value),

	/// A relpath value.
	Relpath(relpath::Value),

	/// A subpath value.
	Subpath(subpath::Value),

	/// A blob value.
	Blob(blob::Value),

	/// A directory value.
	Directory(directory::Value),

	/// A file value.
	File(file::Value),

	/// A symlink value.
	Symlink(symlink::Value),

	/// A placeholder value.
	Placeholder(placeholder::Value),

	/// A template value.
	Template(template::Value),

	/// A package value.
	Package(package::Value),

	/// A resource value.
	Resource(resource::Value),

	/// A target value.
	Target(target::Value),

	/// A task value.
	Task(task::Value),

	/// An array value.
	Array(array::Value),

	/// An object value.
	Object(object::Value),
}

/// Value data.
#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub enum Data {
	#[tangram_serialize(id = 0)]
	Null(null::Data),

	#[tangram_serialize(id = 1)]
	Bool(bool::Data),

	#[tangram_serialize(id = 2)]
	Number(number::Data),

	#[tangram_serialize(id = 3)]
	String(string::Data),

	#[tangram_serialize(id = 4)]
	Bytes(bytes::Data),

	#[tangram_serialize(id = 5)]
	Relpath(relpath::Data),

	#[tangram_serialize(id = 6)]
	Subpath(subpath::Data),

	#[tangram_serialize(id = 7)]
	Blob(blob::Data),

	#[tangram_serialize(id = 8)]
	Directory(directory::Data),

	#[tangram_serialize(id = 9)]
	File(file::Data),

	#[tangram_serialize(id = 10)]
	Symlink(symlink::Data),

	#[tangram_serialize(id = 11)]
	Placeholder(placeholder::Data),

	#[tangram_serialize(id = 12)]
	Template(template::Data),

	#[tangram_serialize(id = 13)]
	Package(package::Data),

	#[tangram_serialize(id = 14)]
	Resource(resource::Data),

	#[tangram_serialize(id = 15)]
	Target(target::Data),

	#[tangram_serialize(id = 16)]
	Task(task::Data),

	#[tangram_serialize(id = 17)]
	Array(array::Data),

	#[tangram_serialize(id = 18)]
	Object(object::Data),
}

impl Value {
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		match data {
			Data::Null(_) => Value::Null(()),
			Data::Bool(data) => Value::Bool(data),
			Data::Number(data) => Value::Number(data),
			Data::String(data) => Value::String(data),
			Data::Bytes(data) => Value::Bytes(data),
			Data::Relpath(data) => Value::Relpath(relpath::Value::from_data(data)),
			Data::Subpath(data) => Value::Subpath(subpath::Value::from_data(data)),
			Data::Blob(data) => Value::Blob(blob::Value::from_data(data)),
			Data::Directory(data) => Value::Directory(directory::Value::from_data(data)),
			Data::File(data) => Value::File(file::Value::from_data(data)),
			Data::Symlink(data) => Value::Symlink(symlink::Value::from_data(data)),
			Data::Placeholder(data) => Value::Placeholder(placeholder::Value::from_data(data)),
			Data::Template(data) => Value::Template(template::Value::from_data(data)),
			Data::Package(data) => Value::Package(package::Value::from_data(data)),
			Data::Resource(data) => Value::Resource(resource::Value::from_data(data)),
			Data::Target(data) => Value::Target(target::Value::from_data(data)),
			Data::Task(data) => Value::Task(task::Value::from_data(data)),
			Data::Array(data) => {
				let value = data
					.into_iter()
					.map(any::Handle::with_id)
					.collect::<Vec<_>>();
				Value::Array(value)
			},
			Data::Object(data) => {
				let value = data
					.into_iter()
					.map(|(key, value)| (key, any::Handle::with_id(value)))
					.collect();
				Value::Object(value)
			},
		}
	}

	#[must_use]
	pub fn to_data(&self) -> Data {
		match self {
			Value::Null(_) => Data::Null(()),
			Value::Bool(value) => Data::Bool(*value),
			Value::Number(value) => Data::Number(*value),
			Value::String(value) => Data::String(value.clone()),
			Value::Bytes(value) => Data::Bytes(value.clone()),
			Value::Relpath(value) => Data::Relpath(value.clone()),
			Value::Subpath(value) => Data::Subpath(value.clone()),
			Value::Blob(value) => Data::Blob(value.to_data()),
			Value::Directory(value) => Data::Directory(value.to_data()),
			Value::File(value) => Data::File(value.to_data()),
			Value::Symlink(value) => Data::Symlink(value.to_data()),
			Value::Placeholder(value) => Data::Placeholder(value.to_data()),
			Value::Template(value) => Data::Template(value.to_data()),
			Value::Package(value) => Data::Package(value.to_data()),
			Value::Resource(value) => Data::Resource(value.to_data()),
			Value::Target(value) => Data::Target(value.to_data()),
			Value::Task(value) => Data::Task(value.to_data()),
			Value::Array(value) => {
				Data::Array(value.iter().map(|handle| handle.expect_id()).collect())
			},
			Value::Object(value) => Data::Object(
				value
					.iter()
					.map(|(key, value)| (key.clone(), value.expect_id()))
					.collect(),
			),
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<crate::Handle> {
		match self {
			Self::Null(_)
			| Self::Bool(_)
			| Self::Number(_)
			| Self::String(_)
			| Self::Bytes(_)
			| Self::Relpath(_)
			| Self::Subpath(_)
			| Self::Placeholder(_) => vec![],
			Self::Blob(blob) => blob.children(),
			Self::Directory(directory) => directory.children(),
			Self::File(file) => file.children(),
			Self::Symlink(symlink) => symlink.children(),
			Self::Template(template) => template.children(),
			Self::Package(package) => package.children(),
			Self::Resource(resource) => resource.children(),
			Self::Target(target) => target.children(),
			Self::Task(task) => task.children(),
			Self::Array(array) => array.iter().map(|value| value.clone().into()).collect(),
			Self::Object(map) => map.values().map(|value| value.clone().into()).collect(),
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
	pub fn children(&self) -> Vec<Id> {
		match self {
			Self::Null(_)
			| Self::Bool(_)
			| Self::Number(_)
			| Self::String(_)
			| Self::Bytes(_)
			| Self::Relpath(_)
			| Self::Subpath(_)
			| Self::Placeholder(_) => vec![],
			Self::Blob(blob) => blob.children(),
			Self::Directory(directory) => directory.children(),
			Self::File(file) => file.children(),
			Self::Symlink(symlink) => symlink.children(),
			Self::Template(template) => template.children(),
			Self::Package(package) => package.children(),
			Self::Resource(resource) => resource.children(),
			Self::Target(target) => target.children(),
			Self::Task(task) => task.children(),
			Self::Array(array) => array.iter().copied().map(Into::into).collect(),
			Self::Object(map) => map.values().copied().map(Into::into).collect(),
		}
	}
}
