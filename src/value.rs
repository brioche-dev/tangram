use crate::{
	array, blob, bool, bytes, directory, error, file, null, number, object, package, placeholder,
	relpath, resource, return_error, string, subpath, symlink, target, task, template, value,
	Client, Id, Kind, Result, WrapErr,
};
use async_recursion::async_recursion;
use byteorder::{ReadBytesExt, WriteBytesExt};
use futures::{stream::FuturesUnordered, TryStreamExt};
use std::sync::Arc;

/// A value handle.
#[derive(Clone, Debug)]
pub struct Handle {
	id: Arc<std::sync::RwLock<Option<Id>>>,
	value: Arc<std::sync::RwLock<Option<Value>>>,
}

/// A value variant.
#[derive(Clone, Debug)]
pub enum Variant {
	Null(crate::Null),
	Bool(crate::Bool),
	Number(crate::Number),
	String(crate::String),
	Bytes(crate::Bytes),
	Relpath(crate::Relpath),
	Subpath(crate::Subpath),
	Blob(crate::Blob),
	Directory(crate::Directory),
	File(crate::File),
	Symlink(crate::Symlink),
	Placeholder(crate::Placeholder),
	Template(crate::Template),
	Package(crate::Package),
	Resource(crate::Resource),
	Target(crate::Target),
	Task(crate::Task),
	Array(crate::Array),
	Object(crate::Object),
}

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

impl Handle {
	#[must_use]
	pub fn with_id(id: Id) -> Self {
		Self {
			id: Arc::new(std::sync::RwLock::new(Some(id))),
			value: Arc::new(std::sync::RwLock::new(None)),
		}
	}

	#[must_use]
	pub fn with_value(value: Value) -> Self {
		Self {
			id: Arc::new(std::sync::RwLock::new(None)),
			value: Arc::new(std::sync::RwLock::new(Some(value))),
		}
	}

	#[must_use]
	pub fn kind(&self) -> Kind {
		if let Some(id) = *self.id.read().unwrap() {
			return id.kind();
		}
		match self.value.read().unwrap().as_ref().unwrap() {
			Value::Null(_) => Kind::Null,
			Value::Bool(_) => Kind::Bool,
			Value::Number(_) => Kind::Number,
			Value::String(_) => Kind::String,
			Value::Bytes(_) => Kind::Bytes,
			Value::Relpath(_) => Kind::Relpath,
			Value::Subpath(_) => Kind::Subpath,
			Value::Blob(_) => Kind::Blob,
			Value::Directory(_) => Kind::Directory,
			Value::File(_) => Kind::File,
			Value::Symlink(_) => Kind::Symlink,
			Value::Placeholder(_) => Kind::Placeholder,
			Value::Template(_) => Kind::Template,
			Value::Package(_) => Kind::Package,
			Value::Resource(_) => Kind::Resource,
			Value::Target(_) => Kind::Target,
			Value::Task(_) => Kind::Task,
			Value::Array(_) => Kind::Array,
			Value::Object(_) => Kind::Object,
		}
	}

	#[must_use]
	pub fn variant(&self) -> value::Variant {
		match self.kind() {
			Kind::Null => value::Variant::Null(self.clone().try_into().unwrap()),
			Kind::Bool => value::Variant::Bool(self.clone().try_into().unwrap()),
			Kind::Number => value::Variant::Number(self.clone().try_into().unwrap()),
			Kind::String => value::Variant::String(self.clone().try_into().unwrap()),
			Kind::Bytes => value::Variant::Bytes(self.clone().try_into().unwrap()),
			Kind::Relpath => value::Variant::Relpath(self.clone().try_into().unwrap()),
			Kind::Subpath => value::Variant::Subpath(self.clone().try_into().unwrap()),
			Kind::Blob => value::Variant::Blob(self.clone().try_into().unwrap()),
			Kind::Directory => value::Variant::Directory(self.clone().try_into().unwrap()),
			Kind::File => value::Variant::File(self.clone().try_into().unwrap()),
			Kind::Symlink => value::Variant::Symlink(self.clone().try_into().unwrap()),
			Kind::Placeholder => value::Variant::Placeholder(self.clone().try_into().unwrap()),
			Kind::Template => value::Variant::Template(self.clone().try_into().unwrap()),
			Kind::Package => value::Variant::Package(self.clone().try_into().unwrap()),
			Kind::Resource => value::Variant::Resource(self.clone().try_into().unwrap()),
			Kind::Target => value::Variant::Target(self.clone().try_into().unwrap()),
			Kind::Task => value::Variant::Task(self.clone().try_into().unwrap()),
			Kind::Array => value::Variant::Array(self.clone().try_into().unwrap()),
			Kind::Object => value::Variant::Object(self.clone().try_into().unwrap()),
		}
	}

	pub(crate) fn expect_id(&self) -> Id {
		self.id.read().unwrap().unwrap()
	}

	pub async fn id(&self, client: &Client) -> Result<Id> {
		// Store the value.
		self.store(client).await?;

		// Return the ID.
		Ok(self.id.read().unwrap().unwrap())
	}

	pub async fn value(&self, client: &Client) -> Result<&Value> {
		// Load the value.
		self.load(client).await?;

		// Return a reference to the value.
		Ok(unsafe { &*(self.value.read().unwrap().as_ref().unwrap() as *const Value) })
	}

	#[allow(clippy::unused_async)]
	pub async fn load(&self, client: &Client) -> Result<()> {
		// If the value is already loaded, then return.
		if self.value.read().unwrap().is_some() {
			return Ok(());
		}

		// Get the id.
		let id = self.id.read().unwrap().unwrap();

		// Get the data.
		let Some(data) = client.try_get_value_bytes(id).await? else {
			return_error!(r#"Failed to find value with id "{id}"."#);
		};

		// Create the value.
		let data = value::Data::deserialize(&data)?;
		let value = Value::from_data(data);

		// Set the value.
		self.value.write().unwrap().replace(value);

		Ok(())
	}

	#[async_recursion]
	pub async fn store(&self, client: &Client) -> Result<()> {
		// If the value is already stored, then return.
		if self.id.read().unwrap().is_some() {
			return Ok(());
		}

		// Store the children.
		let children = self.value.read().unwrap().as_ref().unwrap().children();
		children
			.into_iter()
			.map(|child| async move { child.store(client).await })
			.collect::<FuturesUnordered<_>>()
			.try_collect()
			.await?;

		// Serialize the data.
		let data = self.value.read().unwrap().as_ref().unwrap().to_data();
		let data = data.serialize()?;
		let id = Id::new(self.kind(), &data);

		// Store the value.
		client
			.try_put_value_bytes(id, &data)
			.await
			.wrap_err("Failed to put the value.")?
			.map_err(|_| error!("Expected all children to be stored."))?;

		// Set the ID.
		self.id.write().unwrap().replace(id);

		Ok(())
	}
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
				let value = data.into_iter().map(Handle::with_id).collect::<Vec<_>>();
				Value::Array(value)
			},
			Data::Object(data) => {
				let value = data
					.into_iter()
					.map(|(key, value)| (key, Handle::with_id(value)))
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
			Value::Array(value) => Data::Array(value.iter().map(Handle::expect_id).collect()),
			Value::Object(value) => Data::Object(
				value
					.iter()
					.map(|(key, value)| (key.clone(), value.expect_id()))
					.collect(),
			),
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<Handle> {
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
			Self::Array(array) => array.clone(),
			Self::Object(map) => map.values().cloned().collect(),
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

#[macro_export]
macro_rules! handle {
	($t:ident) => {
		impl self::Handle {
			#[must_use]
			pub fn with_id(id: self::Id) -> Self {
				Self($crate::Handle::with_id(id.into()))
			}

			#[must_use]
			pub fn with_value(value: self::Value) -> Self {
				Self($crate::Handle::with_value(value.into()))
			}

			#[must_use]
			pub fn expect_id(&self) -> self::Id {
				self.0.expect_id().try_into().unwrap()
			}

			pub async fn id(&self, client: &$crate::Client) -> $crate::Result<self::Id> {
				Ok(self.0.id(client).await?.try_into().unwrap())
			}

			pub async fn value(&self, client: &$crate::Client) -> $crate::Result<&self::Value> {
				match self.0.value(client).await? {
					$crate::Value::$t(value) => Ok(value),
					_ => unreachable!(),
				}
			}
		}

		impl From<self::Handle> for $crate::Handle {
			fn from(value: self::Handle) -> Self {
				value.0
			}
		}

		impl TryFrom<$crate::Handle> for self::Handle {
			type Error = $crate::Error;

			fn try_from(value: $crate::Handle) -> Result<Self, Self::Error> {
				match value.kind() {
					$crate::Kind::$t => Ok(Self(value)),
					_ => $crate::return_error!("Unexpected kind."),
				}
			}
		}
	};
}

#[macro_export]
macro_rules! value {
	($t:ident) => {
		impl From<self::Value> for $crate::Value {
			fn from(value: self::Value) -> Self {
				$crate::Value::$t(value)
			}
		}
	};
}
