use crate::{
	blob, directory, file, object, package, return_error, symlink, target, template, Blob,
	Directory, File, Package, Result, Symlink, Target, Template,
};
use bytes::Bytes;
use derive_more::{From, TryInto, TryUnwrap};
use std::collections::BTreeMap;

/// A value.
#[derive(Clone, Debug, From, TryInto, TryUnwrap)]
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

	/// A blob value.
	Blob(Blob),

	/// A directory value.
	Directory(Directory),

	/// A file value.
	File(File),

	/// A symlink value.
	Symlink(Symlink),

	/// A template value.
	Template(Template),

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
	Template(template::Data),

	#[tangram_serialize(id = 10)]
	Package(package::Id),

	#[tangram_serialize(id = 11)]
	Target(target::Id),

	#[tangram_serialize(id = 12)]
	Array(Vec<Data>),

	#[tangram_serialize(id = 13)]
	Map(BTreeMap<String, Data>),
}

impl Value {
	#[must_use]
	pub fn to_data(&self) -> Data {
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
			Value::Template(value) => Data::Template(value.to_data()),
			Value::Package(value) => Data::Package(value.expect_id()),
			Value::Target(value) => Data::Target(value.expect_id()),
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
	pub fn from_data(data: Data) -> Self {
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
			Data::Template(template) => Value::Template(Template::from_data(template)),
			Data::Package(id) => Value::Package(Package::with_id(id)),
			Data::Target(id) => Value::Target(Target::with_id(id)),
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
			Self::Null(_) | Self::Bool(_) | Self::Number(_) | Self::String(_) | Self::Bytes(_) => {
				vec![]
			},
			Self::Blob(blob) => vec![blob.handle().clone()],
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
	pub fn serialize(&self) -> Result<Vec<u8>> {
		let mut bytes = Vec::new();
		byteorder::WriteBytesExt::write_u8(&mut bytes, 0)?;
		tangram_serialize::to_writer(self, &mut bytes)?;
		Ok(bytes)
	}

	pub fn deserialize(mut bytes: &[u8]) -> Result<Self> {
		let version = byteorder::ReadBytesExt::read_u8(&mut bytes)?;
		if version != 0 {
			return_error!(r#"Cannot deserialize with version "{version}"."#);
		}
		let value = tangram_serialize::from_reader(bytes)?;
		Ok(value)
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		match self {
			Self::Null(_) | Self::Bool(_) | Self::Number(_) | Self::String(_) | Self::Bytes(_) => {
				vec![]
			},
			Self::Blob(id) => vec![(*id).into()],
			Self::Directory(id) => vec![(*id).into()],
			Self::File(id) => vec![(*id).into()],
			Self::Symlink(id) => vec![(*id).into()],
			Self::Template(template) => template.children(),
			Self::Package(id) => vec![(*id).into()],
			Self::Target(id) => vec![(*id).into()],
			Self::Array(array) => array.iter().flat_map(Self::children).collect(),
			Self::Map(map) => map.values().flat_map(Self::children).collect(),
		}
	}
}
