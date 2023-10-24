use crate::{
	blob, directory, error, file, object, package, symlink, target, template, Blob, Client,
	Directory, File, Package, Result, Symlink, Target, Template, WrapErr,
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
		byteorder::WriteBytesExt::write_u8(&mut bytes, 0)
			.wrap_err("Failed to write the version.")?;
		tangram_serialize::to_writer(self, &mut bytes).wrap_err("Failed to write the data.")?;
		Ok(bytes)
	}

	pub fn deserialize(mut bytes: &[u8]) -> Result<Self> {
		let version =
			byteorder::ReadBytesExt::read_u8(&mut bytes).wrap_err("Failed to read the version.")?;
		if version != 0 {
			return Err(error!(r#"Cannot deserialize with version "{version}"."#));
		}
		let value = tangram_serialize::from_reader(bytes).wrap_err("Failed to read the data.")?;
		Ok(value)
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		match self {
			Self::Null(_) | Self::Bool(_) | Self::Number(_) | Self::String(_) | Self::Bytes(_) => {
				vec![]
			},
			Self::Blob(id) => vec![id.clone().into()],
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
			Value::Null(_) => Self::Null(()),
			Value::Bool(bool) => Self::Bool(bool),
			Value::Number(number) => Self::Number(number),
			Value::String(string) => Self::String(string.clone()),
			Value::Bytes(bytes) => Self::Bytes(bytes.clone()),
			Value::Blob(blob) => Self::Blob(blob.expect_id().clone()),
			Value::Directory(directory) => Self::Directory(directory.expect_id().clone()),
			Value::File(file) => Self::File(file.expect_id().clone()),
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
			Data::Null(_) => Self::Null(()),
			Data::Bool(bool) => Self::Bool(bool),
			Data::Number(number) => Self::Number(number),
			Data::String(string) => Self::String(string),
			Data::Bytes(bytes) => Self::Bytes(bytes),
			Data::Blob(id) => Self::Blob(Blob::with_id(id)),
			Data::Directory(id) => Self::Directory(Directory::with_id(id)),
			Data::File(id) => Self::File(File::with_id(id)),
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
			Value::Null(_) => {
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
			Value::Blob(blob) => {
				write!(f, "{}", blob.expect_id())?;
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
				write!(f, "`")?;
				for component in template.components() {
					match component {
						template::Component::String(string) => {
							write!(f, "{string}")?;
						},
						template::Component::Artifact(artifact) => {
							write!(f, "${{{}}}", artifact.expect_id())?;
						},
					}
				}
				write!(f, "`")?;
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
