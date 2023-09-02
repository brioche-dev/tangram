use crate::{
	array::Array,
	blob::Blob,
	bool::Bool,
	bytes::Bytes,
	directory::Directory,
	error::{return_error, Error, Result, WrapErr},
	file::File,
	id::Id,
	instance::Instance,
	null::Null,
	number::Number,
	object::Object,
	package::Package,
	placeholder::Placeholder,
	relpath::Relpath,
	resource::Resource,
	string::String,
	subpath::Subpath,
	symlink::Symlink,
	target::Target,
	task::Task,
	template::Template,
	Kind,
};
use byteorder::{ReadBytesExt, WriteBytesExt};
use lmdb::Transaction;
use std::sync::Arc;

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub struct Value {
	#[tangram_serialize(
		id = 0,
		serialize_with = "serialize_id",
		deserialize_with = "deserialize_id"
	)]
	id: Arc<std::sync::RwLock<Option<Id>>>,
	#[tangram_serialize(
		id = 1,
		serialize_with = "serialize_data",
		deserialize_with = "deserialize_data"
	)]
	data: Arc<std::sync::RwLock<Option<Data>>>,
}

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub enum Data {
	#[tangram_serialize(id = 0)]
	Null(Null),

	#[tangram_serialize(id = 1)]
	Bool(Bool),

	#[tangram_serialize(id = 2)]
	Number(Number),

	#[tangram_serialize(id = 3)]
	String(String),

	#[tangram_serialize(id = 4)]
	Bytes(Bytes),

	#[tangram_serialize(id = 5)]
	Relpath(Relpath),

	#[tangram_serialize(id = 6)]
	Subpath(Subpath),

	#[tangram_serialize(id = 7)]
	Blob(Blob),

	#[tangram_serialize(id = 8)]
	Directory(Directory),

	#[tangram_serialize(id = 9)]
	File(File),

	#[tangram_serialize(id = 10)]
	Symlink(Symlink),

	#[tangram_serialize(id = 11)]
	Placeholder(Placeholder),

	#[tangram_serialize(id = 12)]
	Template(Template),

	#[tangram_serialize(id = 13)]
	Package(Package),

	#[tangram_serialize(id = 14)]
	Resource(Resource),

	#[tangram_serialize(id = 15)]
	Target(Target),

	#[tangram_serialize(id = 16)]
	Task(Task),

	#[tangram_serialize(id = 17)]
	Array(Array),

	#[tangram_serialize(id = 18)]
	Object(Object),
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
}

impl Value {
	#[must_use]
	pub fn with_id(id: Id) -> Self {
		Self {
			id: Arc::new(std::sync::RwLock::new(Some(id))),
			data: Arc::new(std::sync::RwLock::new(None)),
		}
	}

	#[must_use]
	pub fn with_data(data: Data) -> Self {
		Self {
			id: Arc::new(std::sync::RwLock::new(None)),
			data: Arc::new(std::sync::RwLock::new(Some(data))),
		}
	}

	pub async fn id(&self, tg: &Instance) -> Result<Id> {
		self.store(tg).await?;
		Ok(self.id.read().unwrap().unwrap())
	}

	pub async fn data(&self, tg: &Instance) -> Result<&Data> {
		self.load(tg).await?;
		Ok(unsafe { &*(self.data.read().unwrap().as_ref().unwrap() as *const Data) })
	}

	#[allow(clippy::unused_async)]
	pub async fn load(&self, tg: &Instance) -> Result<()> {
		// If the value is already loaded, then return.
		if self.data.read().unwrap().is_some() {
			return Ok(());
		}

		// Get the id.
		let id = self.id.read().unwrap().unwrap();

		// Attempt to load the value from the database.
		'a: {
			let txn = tg.database.env.begin_ro_txn()?;
			let data = match txn.get(tg.database.values, &id.as_bytes()) {
				Ok(data) => data,
				Err(lmdb::Error::NotFound) => break 'a,
				Err(error) => return Err(error.into()),
			};
			let data = Data::deserialize(data)?;
			self.data.write().unwrap().replace(data);
			return Ok(());
		}

		// TODO: Attempt to load the value from the parent.

		return_error!("The value was not found.");
	}

	pub async fn store(&self, tg: &Instance) -> Result<()> {
		// If the value is already stored, then return.
		if self.id.read().unwrap().is_some() {
			return Ok(());
		}

		let s = self.clone();
		let tg = tg.clone();
		tokio::task::spawn_blocking(move || {
			// Begin a write transaction.
			let mut txn = tg.database.env.begin_rw_txn()?;

			// Collect the stored values.
			let mut stored = Vec::new();

			// Store the value and its unstored children recursively.
			s.store_inner(&tg, &mut txn, &mut stored)?;

			// Commit the transaction.
			txn.commit()?;

			// Set the ID's of the stored values.
			for (id, value) in stored {
				value.id.write().unwrap().replace(id);
			}

			Ok::<_, Error>(())
		})
		.await
		.map_err(Error::other)
		.wrap_err("Failed to join the store task.")?
		.wrap_err("Failed to store the value.")?;
		Ok(())
	}

	fn store_inner(
		&self,
		tg: &Instance,
		txn: &mut lmdb::RwTransaction,
		stored: &mut Vec<(Id, Value)>,
	) -> Result<()> {
		// If the value is already stored, then return.
		if self.id.read().unwrap().is_some() {
			return Ok(());
		}

		// Otherwise, it must be loaded, so get the data.
		let data = self.data.read().unwrap();
		let data = data.as_ref().unwrap();

		// Store the children.
		for child in data.children() {
			child.store_inner(tg, txn, stored)?;
		}

		// Serialize the data.
		let data = data.serialize()?;
		let id = Id::new(self.kind(), &data);

		// Add the value to the database.
		txn.put(
			tg.database.values,
			&id.as_bytes(),
			&data,
			lmdb::WriteFlags::empty(),
		)?;

		// Add to the stored values.
		stored.push((id, self.clone()));

		Ok(())
	}
}

fn serialize_id<W>(
	id: &Arc<std::sync::RwLock<Option<Id>>>,
	serializer: &mut tangram_serialize::Serializer<W>,
) -> std::io::Result<()>
where
	W: std::io::Write,
{
	serializer.serialize(&*id.read().unwrap())
}

fn deserialize_id<R>(
	deserializer: &mut tangram_serialize::Deserializer<R>,
) -> std::io::Result<Arc<std::sync::RwLock<Option<Id>>>>
where
	R: std::io::Read,
{
	Ok(Arc::new(std::sync::RwLock::new(
		deserializer.deserialize()?,
	)))
}

fn serialize_data<W>(
	data: &Arc<std::sync::RwLock<Option<Data>>>,
	serializer: &mut tangram_serialize::Serializer<W>,
) -> std::io::Result<()>
where
	W: std::io::Write,
{
	serializer.serialize(&*data.read().unwrap())
}

fn deserialize_data<R>(
	deserializer: &mut tangram_serialize::Deserializer<R>,
) -> std::io::Result<Arc<std::sync::RwLock<Option<Data>>>>
where
	R: std::io::Read,
{
	Ok(Arc::new(std::sync::RwLock::new(
		deserializer.deserialize()?,
	)))
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<Value> {
		match self {
			Data::Null(_)
			| Data::Bool(_)
			| Data::Number(_)
			| Data::String(_)
			| Data::Bytes(_)
			| Data::Relpath(_)
			| Data::Subpath(_)
			| Data::Placeholder(_) => vec![],
			Data::Blob(blob) => blob.children(),
			Data::Directory(directory) => directory.children(),
			Data::File(file) => file.children(),
			Data::Symlink(symlink) => symlink.children(),
			Data::Template(template) => template.children(),
			Data::Package(package) => package.children(),
			Data::Resource(resource) => resource.children(),
			Data::Target(target) => target.children(),
			Data::Task(task) => task.children(),
			Data::Array(array) => array.clone(),
			Data::Object(map) => map.values().cloned().collect(),
		}
	}
}

impl Value {
	#[must_use]
	pub fn kind(&self) -> Kind {
		if let Some(id) = *self.id.read().unwrap() {
			return id.kind();
		}
		match self.data.read().unwrap().as_ref().unwrap() {
			Data::Null(_) => Kind::Null,
			Data::Bool(_) => Kind::Bool,
			Data::Number(_) => Kind::Number,
			Data::String(_) => Kind::String,
			Data::Bytes(_) => Kind::Bytes,
			Data::Relpath(_) => Kind::Relpath,
			Data::Subpath(_) => Kind::Subpath,
			Data::Blob(_) => Kind::Blob,
			Data::Directory(_) => Kind::Directory,
			Data::File(_) => Kind::File,
			Data::Symlink(_) => Kind::Symlink,
			Data::Placeholder(_) => Kind::Placeholder,
			Data::Template(_) => Kind::Template,
			Data::Package(_) => Kind::Package,
			Data::Resource(_) => Kind::Resource,
			Data::Target(_) => Kind::Target,
			Data::Task(_) => Kind::Task,
			Data::Array(_) => Kind::Array,
			Data::Object(_) => Kind::Object,
		}
	}
}

impl From<Id> for Value {
	fn from(value: Id) -> Self {
		Self::with_id(value)
	}
}

impl From<Data> for Value {
	fn from(value: Data) -> Self {
		Self::with_data(value)
	}
}

/// Define a value type.
#[macro_export]
macro_rules! value {
	($t:ident) => {
		#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
		#[tangram_serialize(into = "crate::Value", try_from = "crate::Value")]
		pub struct Value($crate::Value);

		impl std::ops::Deref for Value {
			type Target = $crate::Value;

			fn deref(&self) -> &Self::Target {
				&self.0
			}
		}

		impl From<Value> for $crate::Value {
			fn from(value: Value) -> Self {
				value.0
			}
		}

		impl TryFrom<$crate::Value> for Value {
			type Error = $crate::error::Error;

			fn try_from(value: $crate::Value) -> Result<Self, Self::Error> {
				match value.kind() {
					$crate::Kind::$t => Ok(Self(value)),
					_ => Err($crate::error::error!("Expected a string value.")),
				}
			}
		}

		impl From<$t> for $crate::value::Data {
			fn from(value: $t) -> Self {
				Self::$t(value)
			}
		}

		impl From<$t> for $crate::Value {
			fn from(value: $t) -> Self {
				Self::with_data(value.into())
			}
		}

		impl From<$t> for Value {
			fn from(value: $t) -> Self {
				Self(value.into())
			}
		}

		impl Value {
			pub async fn get(&self, tg: &$crate::instance::Instance) -> $crate::error::Result<&$t> {
				match self.0.data(tg).await? {
					$crate::value::Data::$t(value) => Ok(value),
					_ => unreachable!(),
				}
			}

			pub fn with_id(id: $crate::Id) -> $crate::error::Result<Self> {
				$crate::Value::with_id(id).try_into()
			}

			#[must_use]
			pub fn with_data(data: $t) -> Self {
				Self($crate::Value::with_data(data.into()))
			}
		}
	};
}
